use crate::env;
use crate::systemd;

#[derive(thiserror::Error, Debug)]
pub enum ShellError {
    #[error("shell not found for '{0}'")]
    ShellNotFound(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    NixErrno(#[from] nix::errno::Errno),
}

fn get_shell_path(shell: &String) -> Result<String, ShellError> {
    use std::io::BufRead;

    for l in std::io::BufReader::new(std::fs::File::open("/etc/shells")?).lines() {
        let line = l?.clone();
        let p = line.trim();
        if let Some((_, s)) = p.rsplit_once('/') {
            if s == shell {
                return Ok(p.to_string());
            }
        }
    }
    Err(ShellError::ShellNotFound(shell.clone()))
}

pub fn launch_login_shell(bottled_shell_path: &String, shell: &String, args: &Vec<String>) -> Result<(), ShellError> {
    use std::ffi::CString;

    if systemd::is_associated_with_systemd() {
        log::trace!("already associated with bottled systemd");

        log::trace!("releasing privilege");
        nix::unistd::setegid(nix::unistd::getgid()).unwrap();
        nix::unistd::seteuid(nix::unistd::getuid()).unwrap();

        let executable = get_shell_path(&shell)?;
        let mut expanded_args: Vec<CString> = vec![
            CString::new(shell.as_str()).unwrap(),
        ];
        for v in args {
            expanded_args.push(CString::new(v.as_str()).unwrap());
        }

        log::trace!("executing shell: {} {:?}", shell, expanded_args);
        nix::unistd::execv(&CString::new(executable).unwrap(), &expanded_args)?;
    } else {
        let uid = libc::uid_t::from(nix::unistd::getuid());
        let pwent = unsafe { libc::getpwuid(uid) };
        let pw_name = unsafe { std::ffi::CStr::from_ptr((*pwent).pw_name) }.to_str().unwrap();
        log::trace!("username acquired: {}(UID={})", pw_name, uid);

        log::info!("associating with bottled systemd");
        systemd::associate_with_systemd().unwrap();

        let executable = systemd::get_machinectl_bin().unwrap();
        let mut expanded_args: Vec<CString> = vec![
            CString::new("machinectl").unwrap(),
            CString::new("shell").unwrap(),
        ];
        for e in env::get_preserved_env() {
            expanded_args.push(CString::new("-E").unwrap());
            expanded_args.push(CString::new(e).unwrap());
        }
        expanded_args.push(CString::new(format!("{}@.host", pw_name)).unwrap());
        if !args.is_empty() {
            expanded_args.push(CString::new(bottled_shell_path.as_str()).unwrap());
            for v in args {
                expanded_args.push(CString::new(v.as_str()).unwrap());
            }
        }

        log::trace!("launch session: {} {:?}", executable, expanded_args);
        nix::unistd::execv(&CString::new(executable).unwrap(), &expanded_args)?;
    }
    unreachable!();
}
