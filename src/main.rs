use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::parse();
    let _guard = sentry::init((
        opt.client_key,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    loop {
        let http_client = reqwest::Client::new();
        match http_client
            .get(&opt.url)
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

        let _ = tokio::time::sleep(tokio::time::Duration::from_millis(opt.interval)).await;
    }
}

#[derive(Parser, Debug)]
struct Opt {
    /// Sentry client key
    #[arg(short, long, env = "SENTRY_KEY")]
    client_key: String,
    /// Watch interval (ms)
    #[arg(short, long)]
    interval: u64,
    /// Status endpoint
    #[arg(short, long)]
    url: String,
}
