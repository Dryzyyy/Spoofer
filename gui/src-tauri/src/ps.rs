use std::process::Command;

/// Exécute une commande avec timeout anti-freeze : si le processus ne meurt
/// pas dans le délai, on le tue et on retourne (-1, "", "timeout").
/// Les sorties PS étant petites (<64Ko), lire après exit ne bloque pas.
pub fn run_cmd_timeout(mut cmd: Command, secs: u64) -> (i32, String, String) {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    let mut child = match cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return (-1, String::new(), e.to_string()),
    };
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                let mut out = String::new();
                let mut err = String::new();
                if let Some(mut o) = child.stdout.take() {
                    let _ = o.read_to_string(&mut out);
                }
                if let Some(mut e) = child.stderr.take() {
                    let _ = e.read_to_string(&mut err);
                }
                return (
                    st.code().unwrap_or(-1),
                    out.trim().to_string(),
                    err.trim().to_string(),
                );
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (-1, String::new(), "timeout".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return (-1, String::new(), e.to_string()),
        }
    }
}

fn powershell_cmd(script: &str) -> Command {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Exécute PowerShell -NoProfile ... et retourne (code, stdout, stderr).
/// Timeout 25s : un powershell pendu ne doit jamais geler le GUI.
pub fn run_ps(script: &str) -> (i32, String, String) {
    run_cmd_timeout(powershell_cmd(script), 25)
}

/// true si le PID existe encore (OpenProcess QUERY_LIMITED_INFORMATION).
pub fn pid_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) if !h.is_invalid() => {
                let _ = CloseHandle(h);
                true
            }
            _ => false,
        }
    }
}

/// Tue un PID (TerminateProcess). Retourne true si demandé.
pub fn kill_pid(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        match OpenProcess(PROCESS_TERMINATE, false, pid) {
            Ok(h) if !h.is_invalid() => {
                let _ = TerminateProcess(h, 0);
                let _ = CloseHandle(h);
                true
            }
            _ => false,
        }
    }
}

/// Commande simple masquée (ipconfig, taskkill...). Retourne le code.
pub fn run_quiet(program: &str, args: &[&str]) -> i32 {
    run_quiet_timeout(program, args, 120)
}

/// Variante avec timeout explicite (flushdns/taskkill : 20s).
pub fn run_quiet_timeout(program: &str, args: &[&str], secs: u64) -> i32 {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    run_cmd_timeout(cmd, secs).0
}
