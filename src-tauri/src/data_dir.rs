//! 用户数据目录解析。
//!
//! 默认使用「可执行文件所在目录 / userData」，让归档数据跟随程序移动；
//! 用户可通过设置页选择其它目录，选择结果写入 exe 同目录的 user-data-path.txt。
//! 配置文件本身始终留在 exe 同目录，避免出现「找不到配置就找不到数据」的循环依赖。

use std::path::{Path, PathBuf};

use tauri::{Manager, Runtime};

const DATA_DIR_NAME: &str = "userData";
const CONFIG_FILE_NAME: &str = "user-data-path.txt";

/// 可执行文件所在目录。
fn executable_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("无法获取程序所在目录：{error}"))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "无法确定程序所在目录".into())
}

/// 配置文件路径（固定在 exe 同目录）。
fn config_path() -> Result<PathBuf, String> {
    Ok(executable_dir()?.join(CONFIG_FILE_NAME))
}

/// 解析实际使用的数据目录。
pub fn resolve_data_dir() -> Result<PathBuf, String> {
    let exe_dir = executable_dir()?;

    if let Ok(content) = fs_read_to_string(&config_path()?) {
        let raw = content.trim();
        if !raw.is_empty() {
            let configured = PathBuf::from(raw);
            // 相对路径按 exe 目录解析，方便整个文件夹搬运。
            let path = if configured.is_absolute() {
                configured
            } else {
                exe_dir.join(configured)
            };
            fs_create_dir_all(&path)?;
            return Ok(path);
        }
    }

    let path = exe_dir.join(DATA_DIR_NAME);
    fs_create_dir_all(&path)?;
    Ok(path)
}

/// 图片归档目录。
pub fn images_dir() -> Result<PathBuf, String> {
    Ok(resolve_data_dir()?.join("images"))
}

/// 视频缓存目录。
pub fn videos_dir() -> Result<PathBuf, String> {
    Ok(resolve_data_dir()?.join("videos"))
}

/// 数据库文件。
pub fn database_path() -> Result<PathBuf, String> {
    Ok(resolve_data_dir()?.join("qzone-archive.sqlite3"))
}

/// 启动时把数据目录（含 images / videos）加入 asset 协议白名单，
/// 否则 convertFileSrc() 生成的 asset:// URL 会被拒绝加载。
pub fn grant_asset_scope<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let dir = resolve_data_dir()?;
    app.asset_protocol_scope()
        .allow_directory(&dir, true)
        .map_err(|error| format!("无法授权数据目录访问：{error}"))?;
    Ok(())
}

/// 当前数据目录（供设置页展示）。
#[tauri::command]
pub fn get_data_dir() -> Result<String, String> {
    Ok(resolve_data_dir()?.to_string_lossy().into_owned())
}

/// 设置数据目录；传绝对路径或相对 exe 目录的路径。
#[tauri::command]
pub fn set_data_dir<R: Runtime>(app: tauri::AppHandle<R>, path: String) -> Result<String, String> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err("目录不能为空".into());
    }
    let configured = PathBuf::from(raw);
    let target = if configured.is_absolute() {
        configured
    } else {
        executable_dir()?.join(configured)
    };
    fs_create_dir_all(&target)?;
    fs_write_string(&config_path()?, raw.trim())?;
    // 切换到新目录后立即授权，避免图片视频白名单缺失。
    app.asset_protocol_scope()
        .allow_directory(&target, true)
        .map_err(|error| format!("无法授权新数据目录访问：{error}"))?;
    Ok(target.to_string_lossy().into_owned())
}

// 薄封装，便于统一错误文案。
fn fs_read_to_string(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

fn fs_create_dir_all(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| format!("无法创建数据目录：{error}"))
}

fn fs_write_string(path: &Path, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|error| format!("无法保存数据目录配置：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_resolves_against_executable_dir() {
        let exe_dir = executable_dir().unwrap();
        let configured = PathBuf::from("myData");
        let path = if configured.is_absolute() { configured } else { exe_dir.join(configured) };
        assert!(path.ends_with("myData"));
    }
}
