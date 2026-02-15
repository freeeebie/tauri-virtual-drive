use keyring::Entry;

const SERVICE_NAME: &str = "ssh-virtual-drive";

/// Windows Credential Manager에 비밀번호 저장
pub fn save_password(connection_id: &str, password: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, connection_id)
        .map_err(|e| format!("자격 증명 항목 생성 실패: {}", e))?;

    entry
        .set_password(password)
        .map_err(|e| format!("비밀번호 저장 실패: {}", e))
}

/// Windows Credential Manager에서 비밀번호 가져오기
pub fn get_password(connection_id: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(SERVICE_NAME, connection_id)
        .map_err(|e| format!("자격 증명 항목 접근 실패: {}", e))?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("비밀번호 가져오기 실패: {}", e)),
    }
}

/// Windows Credential Manager에서 비밀번호 삭제
pub fn delete_password(connection_id: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, connection_id)
        .map_err(|e| format!("자격 증명 항목 접근 실패: {}", e))?;

    match entry.delete_password() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // 이미 없으면 OK
        Err(e) => Err(format!("비밀번호 삭제 실패: {}", e)),
    }
}
