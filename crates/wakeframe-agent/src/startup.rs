use std::{process::Command, sync::Mutex};

const STARTUP_VALUE_NAME: &str = "WakeFrameAgent";
static STARTUP_LOCK: Mutex<()> = Mutex::new(());

// Store an explicit startup-mode command so sign-in launches can be handled distinctly.
fn current_exe_command() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("Failed to resolve current executable: {}", err))?;
    Ok(format!("\"{}\" startup", exe.display()))
}

// Centralize reg.exe execution so startup registration errors include useful details.
fn run_reg_command(args: &[&str]) -> Result<(), String> {
    let output = Command::new("reg")
        .args(args)
        .output()
        .map_err(|err| format!("Failed to execute reg.exe: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(format!("reg.exe failed: {}", details));
    }

    Ok(())
}

// Register the agent in the current user's Run key.
pub fn ensure_startup_enabled() -> Result<(), String> {
    let _guard = STARTUP_LOCK.lock().unwrap();
    let value = current_exe_command()?;
    run_reg_command(&[
        "add",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        "/v",
        STARTUP_VALUE_NAME,
        "/t",
        "REG_SZ",
        "/d",
        &value,
        "/f",
    ])
}

// Remove the Run-key entry when startup is disabled.
#[cfg(test)]
pub fn ensure_startup_disabled() -> Result<(), String> {
    let _guard = STARTUP_LOCK.lock().unwrap();
    run_reg_command(&[
        "delete",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        "/v",
        STARTUP_VALUE_NAME,
        "/f",
    ])
    .or_else(|err| {
        if err.contains("ERROR: The system was unable to find the specified registry key or value")
        {
            Ok(())
        } else {
            Err(err)
        }
    })
}

// Check that the registered Run-key command points at this executable.
#[cfg(test)]
pub fn is_startup_enabled() -> bool {
    let _guard = STARTUP_LOCK.lock().unwrap();
    let expected = match current_exe_command() {
        Ok(value) => value,
        Err(_) => return false,
    };

    let output = match Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            STARTUP_VALUE_NAME,
        ])
        .output()
    {
        Ok(output) => output,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .any(|line| line.contains(STARTUP_VALUE_NAME) && line.contains(&expected))
}

#[cfg(test)]
mod tests {
    use super::{ensure_startup_disabled, ensure_startup_enabled, is_startup_enabled};

    #[test]
    fn startup_key_is_stable() {
        let _ = ensure_startup_disabled();
        assert!(!is_startup_enabled());
    }

    #[test]
    fn startup_key_can_be_registered_and_detected() {
        let _ = ensure_startup_disabled();
        assert!(ensure_startup_enabled().is_ok());
        assert!(is_startup_enabled());
        let _ = ensure_startup_disabled();
    }
}
