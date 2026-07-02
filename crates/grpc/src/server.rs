pub mod definitions {
    #[path = "parser.rs"]
    pub mod parser;
}

use liteparse::LiteParse;
use liteparse::LiteParseConfig as ImplLiteParseConfig;
use liteparse::OutputFormat;
use liteparse::config::ImageMode;
use liteparse::types::PdfInput;
use tonic::Code;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;
use tonic::transport::Server;
use tracing::{debug, error, info, instrument};

use crate::definitions::parser::LiteParseConfig;
use crate::definitions::parser::PageComplexityStats;
use crate::definitions::parser::ParsedPage;
use crate::definitions::parser::ScreenshotPage;
use crate::definitions::parser::TextItem;
use crate::definitions::parser::parser_service_server::ParserServiceServer;
use crate::definitions::parser::{
    IsComplexRequest, IsComplexResponse, ParseRequest, ParseResponse, ScreenshotRequest,
    ScreenshotResponse, parser_service_server::ParserService,
};

const IMAGE_MIME_TYPE: &str = "image/png";

#[derive(Debug)]
pub struct GrpcService {}

fn int_mode_to_image_mode(image_mode: i32) -> ImageMode {
    match image_mode {
        0 | 1 => ImageMode::Off,
        2 => ImageMode::Placeholder,
        3 => ImageMode::Embed,
        _ => unreachable!("This should never be reached"),
    }
}

fn int_format_to_output_format(output_format: i32) -> OutputFormat {
    match output_format {
        0 | 1 => OutputFormat::Json,
        2 => OutputFormat::Text,
        3 => OutputFormat::Markdown,
        _ => unreachable!("This should never be reached"),
    }
}

impl Into<ImplLiteParseConfig> for LiteParseConfig {
    fn into(self) -> ImplLiteParseConfig {
        ImplLiteParseConfig {
            ocr_enabled: self.ocr_enabled,
            ocr_failure_fatal: self.ocr_failure_fatal,
            ocr_hedge_delays_ms: self.ocr_hedge_delays_ms,
            ocr_language: self.ocr_language,
            ocr_server_headers: self
                .ocr_server_headers
                .iter()
                .map(|h| (h.name.clone(), h.value.clone()))
                .collect(),
            ocr_server_url: self.ocr_server_url,
            tessdata_path: self.tessdata_path,
            target_pages: self.target_pages,
            max_pages: self.max_pages as usize,
            image_mode: int_mode_to_image_mode(self.image_mode),
            emit_word_boxes: self.emit_word_boxes,
            extract_links: self.extract_links,
            num_workers: self.num_workers as usize,
            password: self.password,
            dpi: self.dpi,
            preserve_very_small_text: self.preserve_very_small_text,
            quiet: self.quiet,
            output_format: int_format_to_output_format(self.output_format),
        }
    }
}

#[async_trait]
impl ParserService for GrpcService {
    #[instrument(skip(self, request), fields(file_len))]
    async fn parse(
        &self,
        request: Request<ParseRequest>,
    ) -> Result<Response<ParseResponse>, Status> {
        let request_inner = request.into_inner();
        let file_len = request_inner.file.len();
        tracing::Span::current().record("file_len", file_len);
        info!("parse request received: file_len={}", file_len);

        let config = request_inner
            .config
            .map_or(ImplLiteParseConfig::default(), |c| c.into());
        debug!(?config, "using parse config");

        let lit = LiteParse::new(config);
        let converted = lit
            .parse_input(PdfInput::Bytes(request_inner.file))
            .await
            .map_err(|e| {
                error!(error = %e, "parse failed");
                Status::new(Code::Internal, e.to_string())
            })?;

        let page_count = converted.pages.len();
        info!(page_count, "parse completed successfully");

        Ok(Response::new(ParseResponse {
            text: converted.text,
            pages: converted
                .pages
                .iter()
                .map(|p| ParsedPage {
                    page_number: p.page_number as u32,
                    page_height: p.page_height,
                    page_width: p.page_width,
                    text_items: p
                        .text_items
                        .iter()
                        .map(|t| TextItem {
                            text: t.text.clone(),
                            x: t.x,
                            y: t.y,
                            width: t.width,
                            height: t.height,
                        })
                        .collect(),
                })
                .collect(),
        }))
    }

    #[instrument(skip(self, request), fields(file_len))]
    async fn screenshot(
        &self,
        request: Request<ScreenshotRequest>,
    ) -> Result<Response<ScreenshotResponse>, Status> {
        let request_inner = request.into_inner();
        let file_len = request_inner.file.len();
        tracing::Span::current().record("file_len", file_len);
        info!("screenshot request received: file_len={}", file_len);

        let config = request_inner
            .config
            .map_or(ImplLiteParseConfig::default(), |c| c.into());
        debug!(?config, "using screenshot config");

        let lit = LiteParse::new(config);
        let results = lit
            .screenshot_input(PdfInput::Bytes(request_inner.file), None)
            .await
            .map_err(|e| {
                error!(error = %e, "screenshot failed");
                Status::new(Code::Internal, e.to_string())
            })?;

        let screenshot_count = results.len();
        info!(screenshot_count, "screenshot completed successfully");

        let to_return: Vec<ScreenshotPage> = results
            .iter()
            .map(|r| ScreenshotPage {
                page_number: r.page_num,
                image_bytes: r.image_bytes.clone(),
                height: r.height,
                width: r.width,
                mime_type: IMAGE_MIME_TYPE.to_owned(),
            })
            .collect();
        Ok(Response::new(ScreenshotResponse {
            screenshots: to_return,
        }))
    }

    #[instrument(skip(self, request), fields(file_len))]
    async fn is_complex(
        &self,
        request: Request<IsComplexRequest>,
    ) -> Result<Response<IsComplexResponse>, Status> {
        let request_inner = request.into_inner();
        let file_len = request_inner.file.len();
        tracing::Span::current().record("file_len", file_len);
        info!("is_complex request received: file_len={}", file_len);

        let config = request_inner
            .config
            .map_or(ImplLiteParseConfig::default(), |c| c.into());
        debug!(?config, "using is_complex config");

        let lit = LiteParse::new(config);
        let result = lit
            .is_complex(PdfInput::Bytes(request_inner.file))
            .await
            .map_err(|e| {
                error!(error = %e, "is_complex failed");
                Status::new(Code::Internal, e.to_string())
            })?;

        let page_count = result.len();
        info!(page_count, "is_complex completed successfully");

        let to_return: Vec<PageComplexityStats> = result
            .iter()
            .map(|c| PageComplexityStats {
                page_area: c.page_area,
                page_number: c.page_number as u32,
                full_page_image: c.full_page_image,
                is_garbled: c.is_garbled,
                image_block_count: c.image_block_count as u32,
                image_coverage: c.image_coverage,
                largest_image_coverage: c.largest_image_coverage,
                text_length: c.text_length as u64,
                text_coverage: c.text_coverage,
                has_substantial_images: c.has_substantial_images,
                needs_ocr: c.needs_ocr,
                uncoverted_vector_area: c.uncovered_vector_area,
                reasons: c.reasons.iter().map(|r| r.as_str().to_owned()).collect(),
            })
            .collect();
        Ok(Response::new(IsComplexResponse {
            complexity: to_return,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    observability::init_tracing_subscriber();

    let addr = "0.0.0.0:50051".parse()?;
    info!(%addr, "starting gRPC server");
    let service = GrpcService {};

    Server::builder()
        .add_service(
            ParserServiceServer::new(service)
                .max_decoding_message_size(30 * 1024 * 1024)
                .max_encoding_message_size(30 * 1024 * 1024),
        )
        .serve(addr)
        .await?;

    info!("gRPC server shut down");
    Ok(())
}
