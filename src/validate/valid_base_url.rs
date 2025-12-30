use crate::models::bundle::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let check_base_url =
        format!("https://github.com/hydrolix/integration-deployment-templates/blob/main/{base}");

    if bundle.base_url != check_base_url {
        return Err(format!(
            "ERROR: {}.{} Invalid {} base_url should be this: '{}'",
            file!(),
            line!(),
            bundle.name,
            check_base_url,
        ));
    }

    Ok(())
}
