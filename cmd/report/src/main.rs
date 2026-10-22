//! 本文件提供 HTML/Markdown 报告 CLI 入口和退出错误输出。

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Render an Actrail KV analysis as HTML and/or Markdown"
)]
struct Cli {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Output format. Omit to generate both HTML and Markdown.
    #[arg(long, value_enum)]
    format: Option<FormatArg>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
enum FormatArg {
    Html,
    Md,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let format = cli.format.map(|format| match format {
        FormatArg::Html => actrail_kv_reporter::ReportFormat::Html,
        FormatArg::Md => actrail_kv_reporter::ReportFormat::Markdown,
    });
    actrail_kv_reporter::write_reports(&cli.input, &cli.output, format)
}

#[cfg(test)]
mod tests {
    use super::{Cli, FormatArg};
    use clap::Parser;

    fn base() -> Vec<&'static str> {
        vec![
            "actrail-kv-report",
            "--input",
            "analysis.json",
            "--output",
            "report.html",
        ]
    }

    #[test]
    fn format_is_optional_and_accepts_html_or_md() {
        let default = Cli::try_parse_from(base()).expect("default");
        assert_eq!(default.format, None);

        let mut html = base();
        html.extend(["--format", "html"]);
        assert_eq!(
            Cli::try_parse_from(html).expect("html").format,
            Some(FormatArg::Html)
        );

        let mut markdown = base();
        markdown.extend(["--format", "md"]);
        assert_eq!(
            Cli::try_parse_from(markdown).expect("markdown").format,
            Some(FormatArg::Md)
        );

        let mut invalid = base();
        invalid.extend(["--format", "pdf"]);
        assert!(Cli::try_parse_from(invalid).is_err());
    }
}
