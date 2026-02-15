// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

fn main() {
    // WinFsp DLL 경로 자동 설정 (배포 시 필수)
    setup_winfsp_path();

    tauri_app_lib::run()
}

/// WinFsp 설치 경로를 레지스트리에서 찾아 PATH 환경변수에 추가합니다.
/// 이를 통해 사용자가 별도의 환경변수 설정 없이도 앱을 실행할 수 있습니다.
fn setup_winfsp_path() {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // WinFsp 설치 경로 레지스트리 키 시도
    let winfsp_keys = [
        r"SOFTWARE\WOW6432Node\WinFsp", // 64비트 Windows의 32비트 키 (일반적)
        r"SOFTWARE\WinFsp",             // 32비트 Windows 또는 다른 케이스
    ];

    for key_path in winfsp_keys {
        if let Ok(key) = hklm.open_subkey(key_path) {
            if let Ok(install_dir) = key.get_value::<String, _>("InstallDir") {
                let bin_path = PathBuf::from(&install_dir).join("bin");

                if bin_path.exists() {
                    // 현재 프로세스의 PATH 환경 변수 가져오기
                    if let Ok(current_path) = env::var("PATH") {
                        // 이미 경로가 포함되어 있는지 확인
                        let bin_str = bin_path.to_string_lossy().to_string();
                        if !current_path.contains(&bin_str) {
                            // PATH에 추가
                            let new_path = format!("{};{}", current_path, bin_str);
                            env::set_var("PATH", new_path);
                            println!("WinFsp DLL path added to PATH: {}", bin_str);
                        } else {
                            println!(
                                "WinFsp DLL path already suggests it is in PATH: {}",
                                bin_str
                            );
                        }
                    } else {
                        // PATH가 없으면 새로 설정 (매우 드문 경우)
                        env::set_var("PATH", &bin_path);
                        println!("PATH env var was empty, set to: {:?}", bin_path);
                    }
                    return; // 성공적으로 찾아서 추가했으면 종료
                } else {
                    println!("Warning: WinFsp install dir found at {:?} but bin directory {:?} does not exist.", key_path, bin_path);
                }
            }
        }
    }

    // WinFsp를 찾지 못한 경우 (앱 실행 중 오류 발생 가능성 높음)
    println!("Warning: Could not find WinFsp installation in registry. Checked common locations.");
}
