use anyhow::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    let client_key = sentry::types::Dsn::from_str("https://03a10f4aff7d4891bbdbdcacaea2586d@o4505267115982848.ingest.sentry.io/4505267128041472")?;
    let _ = sentry::init(sentry::ClientOptions {
        dsn: Some(client_key),
        ..Default::default()
    });

    loop {
        let url = "https://dragondev-keeper.sandbox.levana.finance/status";
        let http_client = reqwest::Client::new();
        match http_client
            .get(url)
            .header("accept", "text/html")
            .send()
            .await
        {
            Err(e) => {
                eprintln!("{e:?}");
                sentry::capture_error(&e);
            }
            Ok(response) => {
                let status_page = response.text().await?;
                let xdoc = sxd_html::parse_html(&status_page);
                let xdoc = xdoc.as_document();

                let matches = sxd_xpath::evaluate_xpath(
            &xdoc,
            "/html/body/div/div/div/div/p[starts-with(text(), 'Status: ERROR')]/../pre/text()",
        )?;
                if let sxd_xpath::Value::Nodeset(nodes) = matches {
                    for i in nodes {
                        let msg = i.string_value();
                        eprintln!("{msg}");
                        sentry::capture_message(&msg, sentry::Level::Error);
                    }
                }
                //     "/html/body/div/div/div/div/p[starts-with(text(), 'Status: SUCCESS')]/../pre/text()",
            }
        }

        let _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
