pub mod definitions {
    #[path = "parser.rs"]
    pub mod parser;
}

use std::path::Path;

use clap::{Parser, Subcommand};
use liteparse::{LiteParseConfig as ImplLiteParseConfig, OutputFormat, config::ImageMode};
use tonic::transport::Channel;

use crate::definitions::parser::{
    HttpHeader, IsComplexRequest, LiteParseConfig, ParseRequest, ScreenshotRequest,
};

fn image_mode_to_int(image_mode: ImageMode) -> i32 {
    match image_mode {
        ImageMode::Embed => 3,
        ImageMode::Placeholder => 2,
        ImageMode::Off => 1,
    }
}

fn output_format_to_int(output_format: OutputFormat) -> i32 {
    match output_format {
        OutputFormat::Json => 1,
        OutputFormat::Text => 2,
        OutputFormat::Markdown => 3,
    }
}

impl From<ImplLiteParseConfig> for LiteParseConfig {
    fn from(value: ImplLiteParseConfig) -> Self {
        Self {
            password: value.password,
            max_pages: value.max_pages as u64,
            image_mode: image_mode_to_int(value.image_mode),
            output_format: output_format_to_int(value.output_format),
            ocr_enabled: value.ocr_enabled,
            ocr_failure_fatal: value.ocr_failure_fatal,
            ocr_hedge_delays_ms: value.ocr_hedge_delays_ms,
            ocr_language: value.ocr_language,
            ocr_server_url: value.ocr_server_url,
            ocr_server_headers: value
                .ocr_server_headers
                .iter()
                .map(|(name, value)| HttpHeader {
                    name: name.to_owned(),
                    value: value.to_owned(),
                })
                .collect(),
            emit_word_boxes: value.emit_word_boxes,
            extract_links: value.extract_links,
            num_workers: value.num_workers as u64,
            preserve_very_small_text: value.preserve_very_small_text,
            target_pages: value.target_pages,
            quiet: value.quiet,
            tessdata_path: value.tessdata_path,
            dpi: value.dpi,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse a file and return a JSON array of its pages or text content
    Parse {
        /// Path to the file to parse
        #[arg(long, short)]
        file: String,
        /// Path to the config file containing the LiteParse config
        #[arg(long, short, default_value = None)]
        config_file: Option<String>,
        /// Output plain text instead of markdown
        #[arg(long, short, default_value_t = false)]
        no_markdown: bool,
        /// Print an array of JSON pages instead of text
        #[arg(long, short, default_value_t = false)]
        json: bool,
    },

    /// Screenshot the pages of a PDF files and save them as images
    Screenshot {
        /// Path to the file to screenshot
        #[arg(long, short)]
        file: String,
        /// Path to the config file containing the LiteParse config
        #[arg(long, short, default_value = None)]
        config_file: Option<String>,
        /// Path to the folder where to save screenshots. Defaults to 'imgs/'.
        #[arg(long, short, default_value = None)]
        dest_dir: Option<String>,
    },

    /// Estimate the complexity of a file and the need for OCR when parsing it
    IsComplex {
        /// Path to the file whose complexity we need to estimate
        #[arg(long, short)]
        file: String,
        /// Path to the config file containing the LiteParse config
        #[arg(long, short, default_value = None)]
        config_file: Option<String>,
    },
}

/// liteparse-client: a demo CLI app that interfaces with the LiteParse GRPC server
#[derive(Debug, Parser)]
#[command(version = "0.1.0")]
#[command(name = "liteparse-client")]
#[command(about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
    #[arg(short, long, default_value_t = 50051)]
    port: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr = format!("http://localhost:{:?}", args.port);
    let channel = Channel::from_shared(addr)?.connect().await?;
    let grpc_client = definitions::parser::parser_service_client::ParserServiceClient::new(channel)
        .max_decoding_message_size(30 * 1024 * 1024)
        .max_encoding_message_size(30 * 1024 * 1024);
    match args.cmd {
        Command::Parse {
            file,
            config_file,
            no_markdown,
            json,
        } => {
            let fl = std::fs::read(file)?;
            let mut config = config_file.map_or(ImplLiteParseConfig::default(), |f| {
                let content =
                    std::fs::read_to_string(f).expect("Should be able to read config file");
                let conf: ImplLiteParseConfig = serde_json::from_str(&content)
                    .expect("Content should match the LiteParseConfig schema");
                conf
            });
            if no_markdown {
                config.output_format = OutputFormat::Text
            } else {
                config.output_format = OutputFormat::Markdown
            }
            let response = grpc_client
                .clone()
                .parse(ParseRequest {
                    file: fl,
                    config: Some(LiteParseConfig::from(config)),
                })
                .await?;
            if json {
                println!("{:#?}", response.into_inner().pages)
            } else {
                println!("{}", response.into_inner().text)
            }
        }
        Command::Screenshot {
            file,
            config_file,
            dest_dir,
        } => {
            let fl = std::fs::read(file)?;
            let config = config_file.map_or(ImplLiteParseConfig::default(), |f| {
                let content =
                    std::fs::read_to_string(f).expect("Should be able to read config file");
                let conf: ImplLiteParseConfig = serde_json::from_str(&content)
                    .expect("Content should match the LiteParseConfig schema");
                conf
            });
            let response = grpc_client
                .clone()
                .screenshot(ScreenshotRequest {
                    file: fl,
                    config: Some(LiteParseConfig::from(config)),
                })
                .await?;
            let folder = dest_dir.map_or("imgs/".to_string(), |d| d);
            let p = Path::new(&folder);
            if !p.exists() {
                std::fs::create_dir_all(p)?;
            }
            for s in response.into_inner().screenshots {
                std::fs::write(
                    p.join(format!("page_{:?}.png", s.page_number)),
                    s.image_bytes,
                )?;
            }
        }
        Command::IsComplex { file, config_file } => {
            let fl = std::fs::read(file)?;
            let config = config_file.map_or(ImplLiteParseConfig::default(), |f| {
                let content =
                    std::fs::read_to_string(f).expect("Should be able to read config file");
                let conf: ImplLiteParseConfig = serde_json::from_str(&content)
                    .expect("Content should match the LiteParseConfig schema");
                conf
            });
            let response = grpc_client
                .clone()
                .is_complex(IsComplexRequest {
                    file: fl,
                    config: Some(LiteParseConfig::from(config)),
                })
                .await?;
            println!("{:#?}", response.into_inner().complexity)
        }
    }
    Ok(())
}
