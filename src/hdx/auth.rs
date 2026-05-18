use crate::hdx::CLIENT;

pub async fn get_token() -> Result<String, String> {
    let url = format!("https://{}/config/v1/login", super::cluster());

    let response = match CLIENT
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "username": super::username(),
            "password": super::password(),
        }))
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ));
        }
    };

    let status = response.status();
    let json: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Could not deserialize response (HTTP {}): {e}",
                file!(),
                line!(),
                status
            ));
        }
    };

    // Debug: print what we got back
    if !status.is_success() {
        return Err(format!(
            "ERROR: {}.{} Authentication failed (HTTP {}): {}",
            file!(),
            line!(),
            status,
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{:?}", json))
        ));
    }

    let token = json["auth_token"]["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if token.is_empty() {
        Err(format!(
            "ERROR: {}.{} Could not find token in payload. Response was: {}",
            file!(),
            line!(),
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{:?}", json))
        ))
    } else {
        Ok(token)
    }
}
