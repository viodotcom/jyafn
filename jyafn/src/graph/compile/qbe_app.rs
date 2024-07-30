use rand::random;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::{env, fs, io};

const BIN: &[u8] = include_bytes!("../../../../vendored/qbe/qbe");

static CURRENT_QBE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(Mutex::default);

#[cfg(unix)]
fn load() -> Result<PathBuf, io::Error> {
    use std::os::unix::fs::PermissionsExt;

    let rand = random::<u32>();
    let temp_qbe = env::temp_dir().join(format!("qbe-{rand}"));
    fs::write(&temp_qbe, BIN)?;
    fs::set_permissions(&temp_qbe, fs::Permissions::from_mode(0o755))?;

    Ok(temp_qbe)
}

pub fn get_qbe() -> Result<PathBuf, io::Error> {
    let mut guard = CURRENT_QBE.lock().expect("poisoned");
    if guard.is_none() {
        *guard = Some(load()?);
    }

    if let Some(path) = guard.as_mut() {
        if !path.exists() {
            *guard = Some(load()?);
        }
    }

    Ok(guard.as_ref().expect("qbe path loaded").to_owned())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load() {
        dbg!(load().unwrap());
    }
}
