use std::collections::HashSet;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let opt = Opt::parse();
    let _guard = sentry::init((
        opt.client_key,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));
    sentry::configure_scope(|scope| scope.set_tag("bot-name", &opt.bot_name));
    let http_client = reqwest::blocking::Client::new();
    let xpath_factory = sxd_xpath::Factory::new();
    let h2 = xpath_factory.build("//h2/text()")?.unwrap();
    let pre = xpath_factory.build("//pre/text()")?.unwrap();
    let context = sxd_xpath::Context::new();
    let mut states: HashSet<(String, String)> = HashSet::new();

    loop {
        match http_client
            .get(&opt.url)
            .header("accept", "text/html")
            .send()
        {
            Err(e) => {
                eprintln!("{e:?}");
                sentry::capture_message(
                    &format!("Fetching status page error: {e:?}"),
                    sentry::Level::Error,
                );
            }
            Ok(response) => {
                let status_page = response.text()?;
                let xdoc = sxd_html::parse_html(&status_page);
                let xdoc = xdoc.as_document();

                let matches = sxd_xpath::evaluate_xpath(
                    &xdoc,
                    "/html/body/div/div/div/div/p[starts-with(text(), 'Status: ERROR')]/..",
                )?;
                if let sxd_xpath::Value::Nodeset(nodes) = matches {
                    let errors: HashSet<(String, String)> = nodes
                        .iter()
                        .map(|i| {
                            let title =
                                if let sxd_xpath::Value::Nodeset(h2s) = h2.evaluate(&context, i)? {
                                    h2s.iter()
                                        .next()
                                        .map(|x| x.string_value())
                                        .unwrap_or_default()
                                } else {
                                    "".to_string()
                                };
                            let msg = if let sxd_xpath::Value::Nodeset(pres) =
                                pre.evaluate(&context, i)?
                            {
                                pres.iter()
                                    .next()
                                    .map(|x| x.string_value())
                                    .unwrap_or_default()
                            } else {
                                "".to_string()
                            };
                            eprintln!("{title}: {msg}");
                            Ok((title, msg))
                        })
                        .collect::<Result<_>>()?;

                    // New errors
                    for (title, msg) in errors.difference(&states) {
                        sentry::with_scope(
                            |scope| scope.set_tag("part-name", title),
                            || {
                                sentry::capture_message(
                                    &format!("{title}: {msg}"),
                                    sentry::Level::Error,
                                )
                            },
                        );
                    }
                    // Gone errors
                    for (title, msg) in states.difference(&errors) {
                        sentry::with_scope(
                            |scope| scope.set_tag("part-name", title),
                            || {
                                sentry::capture_message(
                                    &format!("Recoverred {title}: {msg}"),
                                    sentry::Level::Info,
                                )
                            },
                        );
                    }

                    states = errors;
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(opt.interval));
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
    /// Bot name
    #[arg(short, long)]
    bot_name: String,
}
