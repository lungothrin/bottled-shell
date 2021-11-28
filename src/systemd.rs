use crate::env;

static RUN_DIR: &str = "/run/bottled-shell";
static PID_FILE: &str = "/run/bottled-shell/systemd.pid";

#[derive(thiserror::Error, Debug)]
pub enum SystemdError {
    #[error("systemd not found in standard locations")]
    SystemdNotFound,

    #[error("systemd not running")]
    SystemdNotRunning,

    #[error("no enough permission, required seteuid")]
    NoEnoughPermission,

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    FromUTF8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    NixErrno(#[from] nix::errno::Errno),
}

fn check_permission() -> Result<bool, SystemdError> {
    if nix::unistd::geteuid().is_root() {
        return Ok(true)
    }
    log::error!("no enough permission, required seteuid on program");
    Err(SystemdError::NoEnoughPermission)
}

fn check_systemd_proc(pid: libc::pid_t) -> bool {
    let path = std::format!("/proc/{}/cmdline", pid);
    if let Ok(buffer) = std::fs::read(path.clone()) {
        let cmdline = String::from_utf8(buffer).unwrap();
        if cmdline.split('\0').next().unwrap() == get_systemd_bin().unwrap() {
            log::trace!("check {}: match", path);
            return true;
        }
    }
    log::trace!("check {}: mismatch", path);
    false
}

pub fn is_associated_with_systemd() -> bool {
    check_systemd_proc(1)
}

fn get_systemd_bin() -> Result<String, SystemdError> {
    let search_location = [
        "/lib/systemd/systemd",
        "/usr/lib/systemd/systemd",
    ];
    for l in search_location {
        if std::fs::metadata(l).is_ok() {
            return Ok(l.to_string());
        }
    }
    Err(SystemdError::SystemdNotFound)
}

pub fn get_machinectl_bin() -> Result<String, SystemdError> {
    let search_location = [
        "/usr/bin/machinectl",
        "/bin/machinectl",
    ];
    for l in search_location {
        if std::fs::metadata(l).is_ok() {
            return Ok(l.to_string());
        }
    }
    Err(SystemdError::SystemdNotFound)
}

pub fn get_systemd_pid() -> Result<Option<libc::pid_t>, SystemdError> {
    let buffer = std::fs::read(PID_FILE);
    match buffer {
        Ok(b) => {
            let pid = String::from_utf8(b)?.trim().parse()?;
            log::trace!("check {}: PID={}", PID_FILE, pid);
            if check_systemd_proc(pid) {
                return Ok(Some(pid));
            }
            Ok(None)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                log::trace!("check {}: missing", PID_FILE);
                Ok(None)
            } else {
                Err(SystemdError::IOError(e))
            }
        }
    }
}

fn put_systemd_pid(pid: libc::pid_t) -> std::io::Result<()> {
    std::fs::create_dir_all(RUN_DIR)?;
    std::fs::write(PID_FILE, format!("{}\n", pid))
}

fn updated_systemd_envs() -> std::io::Result<()> {
    let envs = env::get_preserved_env().join(" ");
    let config = format!("[Manager]\nDefaultEnvironment={}\n", envs);
    log::trace!("updating systemd environment variables: {}", envs);

    std::fs::create_dir_all("/run/systemd/system.conf.d")?;
    std::fs::write("/run/systemd/system.conf.d/10-bottled-shell-env.conf", &config)?;

    std::fs::create_dir_all("/run/systemd/user.conf.d")?;
    std::fs::write("/run/systemd/user.conf.d/10-bottled-shell-env.conf", &config)?;

    Ok(())
}

pub fn start_systemd() -> Result<(), SystemdError> {
    use nix::fcntl::OFlag;
    use nix::poll::PollFlags;
    use nix::sched::CloneFlags;
    use nix::unistd::ForkResult;

    if is_associated_with_systemd() {
        log::info!("systemd already started");
        return Ok(());
    }

    if let Ok(Some(pid)) = get_systemd_pid() {
        log::info!("systemd already started, PID={}", pid);
        return Ok(());
    }

    check_permission()?;

    let systemd_bin = std::ffi::CString::new(get_systemd_bin().unwrap()).unwrap();
    log::trace!("systemd location = {}", systemd_bin.to_str().unwrap());

    updated_systemd_envs().unwrap();

    let (rfd, wfd) = nix::unistd::pipe2(OFlag::O_CLOEXEC).unwrap();
    match unsafe { nix::unistd::fork() } {
        Ok(ForkResult::Parent { .. }) => {
            nix::unistd::close(wfd).unwrap();

            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(10);
            let mut fds = [nix::poll::PollFd::new(rfd, PollFlags::POLLIN)];
            loop {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    log::error!("systemd not started in time");
                    return Err(SystemdError::SystemdNotRunning)
                }

                let remaining = timeout - elapsed;
                if nix::poll::poll(&mut fds, remaining.as_millis() as libc::c_int)? > 0 {
                    if let Some(ev) = fds[0].revents() {
                        if ev.contains(PollFlags::POLLHUP) {
                            if let Ok(Some(pid)) = get_systemd_pid() {
                                log::info!("systemd(PID={}) started", pid);

                                log::trace!("sending SIGRTMIN + 0 to systemd(PID={})", pid);
                                unsafe { nix::libc::kill(pid, libc::SIGRTMIN() + 0); }
                                std::thread::sleep(std::time::Duration::from_secs(1));

                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
        Ok(ForkResult::Child) => {
            nix::unistd::close(rfd).unwrap();

            log::trace!("updating UID & GID");
            nix::unistd::setgid(nix::unistd::Gid::from_raw(0)).unwrap();
            nix::unistd::setuid(nix::unistd::Uid::from_raw(0)).unwrap();

            log::trace!("creating new namespace");
            nix::sched::unshare(CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID).unwrap();

            log::trace!("creating new session group");
            nix::unistd::setsid().unwrap();

            match unsafe { nix::unistd::fork() } {
                Ok(ForkResult::Parent { child, .. }) => {
                    nix::unistd::close(wfd).unwrap();

                    log::trace!("updating PID file with {}", child);
                    put_systemd_pid(libc::pid_t::from(child)).unwrap();

                    log::trace!(
                        "session group leader(PID={}) terminated successfully",
                        nix::unistd::getpid()
                    );
                    std::process::exit(libc::EXIT_SUCCESS);
                }
                Ok(ForkResult::Child) => {
                    exec_systemd(systemd_bin);
                    std::process::exit(libc::EXIT_FAILURE);
                }
                Err(e) => Err(SystemdError::NixErrno(e))
            }
        }
        Err(e) => Err(SystemdError::NixErrno(e))
    }
}

fn exec_systemd(systemd_bin: std::ffi::CString) {
    use std::ffi::OsStr;
    use nix::fcntl::OFlag;
    use nix::mount::MsFlags;
    use nix::sys::stat::Mode;

    log::trace!("mounting filesystem");
    nix::mount::mount(
        Some(OsStr::new("none")),
        OsStr::new("/"),
        None as Option<&[u8]>,
        MsFlags::MS_REC | MsFlags::MS_SHARED,
        None as Option<&[u8]>,
    )
    .unwrap();
    nix::mount::mount(
        Some(OsStr::new("none")),
        OsStr::new("/proc"),
        None as Option<&[u8]>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None as Option<&[u8]>,
    )
    .unwrap();
    nix::mount::mount(
        Some(OsStr::new("proc")),
        OsStr::new("/proc"),
        Some(OsStr::new("proc")),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
        None as Option<&[u8]>,
    )
    .unwrap();

    log::trace!("switch working directory");
    nix::unistd::chdir("/").unwrap();

    log::trace!("updating STDIN, STDOUT, STDERR");
    {
        let fd = nix::fcntl::open(OsStr::new("/dev/null"), OFlag::O_RDONLY, Mode::empty()).unwrap();
        nix::unistd::dup2(fd, libc::STDIN_FILENO).unwrap();
        nix::unistd::close(fd).unwrap();
    }
    {
        let fd = nix::fcntl::open(OsStr::new("/dev/null"), OFlag::O_WRONLY, Mode::empty()).unwrap();
        nix::unistd::dup2(fd, libc::STDOUT_FILENO).unwrap();
        nix::unistd::close(fd).unwrap();
    }
    {
        let fd = nix::fcntl::open(OsStr::new("/dev/null"), OFlag::O_WRONLY, Mode::empty()).unwrap();
        nix::unistd::dup2(fd, libc::STDERR_FILENO).unwrap();
        nix::unistd::close(fd).unwrap();
    }

    log::trace!("launching systemd");
    nix::unistd::execve(systemd_bin.as_c_str(), &[systemd_bin.as_c_str()], &[] as &[std::ffi::CString]).unwrap();
}

pub fn stop_systemd() -> Result<(), SystemdError> {
    if is_associated_with_systemd() {
        kill_systemd(1)
    } else if let Some(pid) = get_systemd_pid()? {
        kill_systemd(pid)
    } else {
        log::info!("systemd not running");
        Ok(())
    }
}

fn kill_systemd(pid: libc::pid_t) -> Result<(), SystemdError> {
    check_permission()?;

    log::trace!("sending SIGRTMIN + 4 to systemd(PID={})", pid);
    unsafe { nix::libc::kill(pid, libc::SIGRTMIN() + 4); }

    if std::fs::metadata(PID_FILE).is_ok() {
        log::trace!("removing {}", PID_FILE);
        std::fs::remove_file(PID_FILE)?;
    }

    log::info!("systemd stopped");
    Ok(())
}

pub fn associate_with_systemd() -> Result<(), SystemdError> {
    use std::ffi::OsStr;
    use nix::fcntl::OFlag;
    use nix::sys::stat::Mode;
    use nix::sched::CloneFlags;

    check_permission()?;

    match get_systemd_pid() {
        Ok(Some(pid)) => {
            log::trace!("associating PID namespace");
            {
                let path = format!("/proc/{}/ns/pid", pid);
                let fd = nix::fcntl::open(OsStr::new(&path), OFlag::O_RDONLY, Mode::empty())?;
                nix::sched::setns(fd, CloneFlags::CLONE_NEWPID)?;
                nix::unistd::close(fd)?;
            }

            log::trace!("associating MNT namespace");
            {
                let path = format!("/proc/{}/ns/mnt", pid);
                let fd = nix::fcntl::open(OsStr::new(&path), OFlag::O_RDONLY, Mode::empty())?;
                nix::sched::setns(fd, CloneFlags::CLONE_NEWNS)?;
                nix::unistd::close(fd)?;
            }

            log::trace!("switch working directory");
            nix::unistd::chdir("/").unwrap();

            Ok(())
        },
        Ok(None) => Err(SystemdError::SystemdNotRunning),
        Err(e) => Err(e),
    }
}
