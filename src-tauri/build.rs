fn main() {
    // WinFsp DLL 지연 로딩 설정 (앱 시작 시 DLL이 없어도 실행되도록 함)
    // 이를 통해 main 함수에서 PATH를 수정한 후 DLL을 로드할 수 있음
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        println!("cargo:rustc-link-arg=/DELAYLOAD:winfsp-x64.dll");
    } else if cfg!(all(target_os = "windows", target_arch = "x86")) {
        println!("cargo:rustc-link-arg=/DELAYLOAD:winfsp-x86.dll");
    }

    tauri_build::build()
}
