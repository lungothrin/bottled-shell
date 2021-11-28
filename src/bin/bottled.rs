use bottled_shell::systemd;
use bottled_shell::shell;

fn main() {
    if let Err(_) = std::env::var("BOTTLED_SHELL_LOG") {
        std::env::set_var("BOTTLED_SHELL_LOG", "info");
    }
    pretty_env_logger::init_custom_env("BOTTLED_SHELL_LOG");

    let mut app = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(
            clap::SubCommand::with_name("is-inside")
                .about("Return 0 if invoked from inside of a bottled systemd environment")
        )
        .subcommand(
            clap::SubCommand::with_name("is-running")
                .about("Return 0 if a bottled systemd environment is running")
        )
        .subcommand(
            clap::SubCommand::with_name("start")
                .about("Start a bottled systemd environment")
        )
        .subcommand(
            clap::SubCommand::with_name("stop")
                .about("Start running bottled systemd environment")
        )
        .subcommand(
            clap::SubCommand::with_name("shell")
                .about("Start running bottled systemd environment")
                .arg(
                    clap::Arg::with_name("shell")
                        .short("s")
                        .long("shell")
                        .value_name("SHELL")
                        .help("Specify interactive shell")
                        .takes_value(true)
                )
                .arg(
                    clap::Arg::with_name("shell-options")
                        .raw(true)
                )
        );
    let matches = app.get_matches_from_safe_borrow(std::env::args_os()).unwrap_or_else(|e| {
        if e.use_stderr() {
            eprintln!("{}", e.message);
            std::process::exit(1);
        }
        e.exit()
    });

    match matches.subcommand() {
        ("is-inside", _) => {
            if systemd::is_associated_with_systemd() {
                log::info!("is-inside=true");
                std::process::exit(libc::EXIT_SUCCESS);
            } else {
                log::info!("is-inside=false");
                std::process::exit(libc::EXIT_FAILURE);
            }
        }
        ("is-running", _) => {
            if systemd::is_associated_with_systemd() {
                log::info!("is-running=true");
                std::process::exit(libc::EXIT_SUCCESS);
            } else if let Ok(Some(pid)) = systemd::get_systemd_pid() {
                log::info!("is-running=true, PID={}", pid);
                std::process::exit(libc::EXIT_SUCCESS);
            } else {
                log::info!("is-running=false");
                std::process::exit(libc::EXIT_FAILURE);
            }
        }
        ("start", _) => {
            systemd::start_systemd().unwrap();
        }
        ("stop", _) => {
            systemd::stop_systemd().unwrap();
        }
        ("shell", Some(m)) => {
            let mut shell = "bash";
            if let Some(s) = m.value_of("shell") {
                shell = s.clone();
            };
            log::debug!("specified shell: {}", shell);

            let bottled_shell = if let Some((c, _)) = clap::crate_name!().rsplit_once('-') {
                format!("{}-{}", c, shell)
            } else {
                clap::crate_name!().to_string()
            };
            let bottled_shell_path = std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from(app.get_bin_name().unwrap()))
                .parent().unwrap()
                .join(bottled_shell)
                .to_str().unwrap()
                .to_string();

            let mut args: Vec<String> = Vec::new();
            for v in m.values_of_lossy("shell-options").unwrap_or(Vec::new()) {
                args.push(v);
            }

            if !systemd::is_associated_with_systemd() && None == systemd::get_systemd_pid().unwrap() {
                log::trace!("starting bottled systemd");
                systemd::start_systemd().unwrap();
            }

            log::trace!("starting login shell: {}", shell);
            shell::launch_login_shell(&bottled_shell_path, &shell.to_string(), &args).unwrap();
        }
        _ => unreachable!()
    }
}
