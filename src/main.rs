use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, ValueEnum};
use reqwest::blocking::Client;
use serde_json::Value;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

const BROWSER_UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:151.0) Gecko/20100101 Firefox/151.0";
const COOKIE: LazyLock<Option<String>> = LazyLock::new(|| std::env::var("REDDIT_COOKIE").ok());

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Sort {
    Hot,
    New,
    Top,
    Rising,
    Controversial,
}
impl Sort {
    fn as_str(&self) -> &'static str {
        match self {
            Sort::Hot => "hot",
            Sort::New => "new",
            Sort::Top => "top",
            Sort::Rising => "rising",
            Sort::Controversial => "controversial",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Time {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}
impl Time {
    fn as_str(&self) -> &'static str {
        match self {
            Time::Hour => "hour",
            Time::Day => "day",
            Time::Week => "week",
            Time::Month => "month",
            Time::Year => "year",
            Time::All => "all",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum UserCat {
    Overview,
    Submitted,
    Comments,
    Gilded,
}
impl UserCat {
    fn as_str(&self) -> &'static str {
        match self {
            UserCat::Overview => "overview",
            UserCat::Submitted => "submitted",
            UserCat::Comments => "comments",
            UserCat::Gilded => "gilded",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FrontList {
    Best,
    Hot,
    New,
    Top,
    Rising,
    Random,
    Popular,
    All,
}
impl FrontList {
    fn as_str(&self) -> &'static str {
        match self {
            FrontList::Best => "best",
            FrontList::Hot => "hot",
            FrontList::New => "new",
            FrontList::Top => "top",
            FrontList::Rising => "rising",
            FrontList::Random => "random",
            FrontList::Popular => "popular",
            FrontList::All => "all",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "reddit",
    version,
    about = "Reddit CLI - Browse Reddit via the public JSON API",
    long_about = None,
    styles = STYLES,
)]
struct Cli {
    #[arg(short = 's', long = "sub", help = "Subreddit name (without r/)")]
    sub: Option<String>,

    #[arg(
        short = 'p',
        long = "pg",
        default_value = "1",
        help = "Page number for results (walks via after cursor)"
    )]
    pg: u32,

    #[arg(
        short = 'L',
        long = "limit",
        default_value = "25",
        help = "Items per request (1-100)"
    )]
    limit: u32,

    #[arg(
        short = 'S',
        long = "sort",
        value_enum,
        default_value = "hot",
        help = "Listing sort [possible values: hot, new, top, rising, controversial]"
    )]
    sort: Sort,

    #[arg(
        short = 't',
        long = "time",
        value_enum,
        default_value = "all",
        help = "Time window for top/controversial [possible values: hour, day, week, month, year, all]"
    )]
    time: Time,

    #[arg(short = 'q', long = "search", help = "Search Reddit")]
    search: Option<String>,

    #[arg(short = 'u', long = "user", help = "View user profile / activity")]
    user: Option<String>,

    #[arg(
        short = 'U',
        long = "ucat",
        value_enum,
        default_value = "overview",
        help = "User listing category [possible values: overview, submitted, comments, gilded]"
    )]
    ucat: UserCat,

    #[arg(
        short = 'i',
        long = "post",
        help = "Get post details and comments (fullname t3_xxx or 6-char id)"
    )]
    post: Option<String>,

    #[arg(
        short = 'C',
        long = "comment",
        help = "Focus on specific comment id (use with --post)"
    )]
    comment: Option<String>,

    #[arg(
        short = 'D',
        long = "download",
        help = "Download media from a post (use with --post)"
    )]
    download: bool,

    #[arg(
        long = "dir",
        default_value = ".",
        help = "Output directory for downloaded media"
    )]
    dir: String,

    #[arg(
        short = 'l',
        long = "list",
        value_enum,
        help = "Front page listing [possible values: best, hot, new, top, rising, random, popular, all]"
    )]
    list: Option<FrontList>,

    #[arg(
        short = 'a',
        long = "after",
        help = "'after' fullname for pagination (e.g. t3_xxxxxx)"
    )]
    after: Option<String>,

    #[arg(
        short = 'b',
        long = "before",
        help = "'before' fullname for pagination"
    )]
    before: Option<String>,

    #[arg(long = "raw", help = "Output raw compact JSON")]
    raw: bool,

    #[arg(
        long = "compact",
        help = "Only output children/results array (unwrapped from listings)"
    )]
    compact: bool,
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn base() -> &'static str {
    "https://www.reddit.com"
}

fn finalize(path: &str, params: &[(String, String)]) -> String {
    let path = if path.ends_with(".json") {
        path.to_string()
    } else {
        format!("{}.json", path)
    };
    let mut url = format!("{}{}", base(), path);
    if !params.is_empty() {
        let qs: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(v)))
            .collect();
        url.push('?');
        url.push_str(&qs.join("&"));
    }
    url
}

fn build_url(cli: &Cli, limit: u32, after: Option<&str>, before: Option<&str>) -> String {
    let mut params: Vec<(String, String)> = Vec::new();

    let add_paging = |params: &mut Vec<(String, String)>| {
        params.push(("limit".into(), limit.to_string()));
        if let Some(a) = after {
            params.push(("after".into(), a.into()));
        }
        if let Some(b) = before {
            params.push(("before".into(), b.into()));
        }
    };

    if let Some(post) = &cli.post {
        let id = post.trim_start_matches("t3_");
        let path = if let Some(sub) = &cli.sub {
            format!("/r/{}/comments/{}/", sub, id)
        } else {
            format!("/comments/{}/", id)
        };
        if let Some(c) = &cli.comment {
            let c = c.trim_start_matches("t1_");
            params.push(("comment".into(), c.into()));
        }
        return finalize(&path, &params);
    }

    if let Some(user) = &cli.user {
        let path = format!("/user/{}/{}", user, cli.ucat.as_str());
        params.push(("sort".into(), cli.sort.as_str().into()));
        if matches!(cli.sort, Sort::Top | Sort::Controversial) {
            params.push(("t".into(), cli.time.as_str().into()));
        }
        add_paging(&mut params);
        return finalize(&path, &params);
    }

    if let Some(q) = &cli.search {
        let path = if let Some(sub) = &cli.sub {
            params.push(("restrict_sr".into(), "1".into()));
            params.push(("include_over_18".into(), "1".into()));
            format!("/r/{}/search", sub)
        } else {
            "/search".to_string()
        };
        params.push(("q".into(), q.clone()));
        params.push(("sort".into(), cli.sort.as_str().into()));
        params.push(("t".into(), cli.time.as_str().into()));
        add_paging(&mut params);
        return finalize(&path, &params);
    }

    if let Some(sub) = &cli.sub {
        let path = format!("/r/{}/{}", sub, cli.sort.as_str());
        if matches!(cli.sort, Sort::Top | Sort::Controversial) {
            params.push(("t".into(), cli.time.as_str().into()));
        }
        add_paging(&mut params);
        return finalize(&path, &params);
    }

    if let Some(list) = &cli.list {
        let path = format!("/{}", list.as_str());
        if matches!(list, FrontList::Top) {
            params.push(("t".into(), cli.time.as_str().into()));
        }
        add_paging(&mut params);
        return finalize(&path, &params);
    }

    let path = "/".to_string();
    add_paging(&mut params);
    finalize(&path, &params)
}

fn fetch(url: &str) -> Result<Value, String> {
    let has_cookie = COOKIE.as_deref().is_some_and(|c| !c.is_empty());

    let client = Client::builder()
        .user_agent(BROWSER_UA)
        .timeout(Duration::from_secs(30))
        .gzip(true)
        .build()
        .map_err(|e| format!("client build: {}", e))?;

    let mut req = client.get(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9");

    if has_cookie {
        req = req.header("Cookie", COOKIE.as_deref().unwrap());
    }

    let resp = req.send().map_err(|e| format!("request: {}", e))?;

    let status = resp.status();
    let text = resp.text().map_err(|e| format!("read body: {}", e))?;

    if !status.is_success() {
        let snippet: String = text.chars().take(200).collect();
        let mut msg = format!("HTTP {}: {}", status, snippet);
        if status.as_u16() == 403 {
            msg.push_str(
                "\n\nThe Reddit JSON API now requires authentication.\n\
                 Set REDDIT_COOKIE with your browser session cookie.",
            );
        }
        return Err(msg);
    }
    serde_json::from_str::<Value>(&text).map_err(|e| format!("parse JSON: {}", e))
}

fn extract_after(v: &Value) -> Option<String> {
    v.get("data")?.get("after")?.as_str().map(|s| s.to_string())
}

fn compact_value(v: &Value) -> Value {
    if let Some(children) = v
        .get("data")
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
    {
        let arr: Vec<Value> = children
            .iter()
            .filter_map(|c| c.get("data").cloned())
            .collect();
        return Value::Array(arr);
    }
    if let Some(arr) = v.as_array() {
        if arr.len() >= 2 {
            if let Some(children) = arr[1]
                .get("data")
                .and_then(|d| d.get("children"))
                .and_then(|c| c.as_array())
            {
                let items: Vec<Value> = children
                    .iter()
                    .filter_map(|c| c.get("data").cloned())
                    .collect();
                return Value::Array(items);
            }
        }
    }
    v.clone()
}

fn print_output(v: &Value, cli: &Cli) {
    let target = if cli.compact {
        compact_value(v)
    } else {
        v.clone()
    };
    let out = if cli.raw {
        serde_json::to_string(&target).unwrap_or_else(|_| "{}".into())
    } else {
        serde_json::to_string_pretty(&target).unwrap_or_else(|_| "{}".into())
    };
    println!("{}", out);
}

fn find_media_url(post_data: &Value) -> Option<(String, &'static str)> {
    let domain = post_data.get("domain")?.as_str()?;

    if domain == "i.redd.it" {
        let url = post_data.get("url")?.as_str()?;
        return Some((url.to_string(), "image"));
    }

    if domain == "v.redd.it" {
        for key in &["secure_media", "media"] {
            if let Some(url) = post_data[key]["reddit_video"]["fallback_url"].as_str() {
                let clean = url.split('?').next().unwrap_or(url);
                return Some((clean.to_string(), "video"));
            }
        }
    }

    if let Some(url) = post_data["preview"]["reddit_video_preview"]["fallback_url"].as_str() {
        let clean = url.split('?').next().unwrap_or(url);
        return Some((clean.to_string(), "video"));
    }

    let url = post_data.get("url")?.as_str()?;
    Some((url.to_string(), "link"))
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let client = Client::builder()
        .user_agent(BROWSER_UA)
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("client build: {}", e))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("download request: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {} downloading {}", status, url));
    }
    let bytes = resp.bytes().map_err(|e| format!("read body: {}", e))?;
    std::fs::write(dest, &bytes).map_err(|e| format!("write file: {}", e))?;
    Ok(())
}

fn download_gallery(post_data: &Value, dir: &Path, post_id: &str) {
    let metadata = match post_data.get("media_metadata").and_then(|m| m.as_object()) {
        Some(m) => m,
        None => return,
    };

    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("warning: cannot create directory: {}", e);
        return;
    }

    let ordered_ids: Vec<String> = if let Some(arr) =
        post_data["gallery_data"]["items"].as_array()
    {
        arr.iter()
            .filter_map(|item| item.get("media_id").and_then(|m| m.as_str()))
            .map(|s| s.to_string())
            .collect()
    } else {
        // fallback: use metadata keys sorted alphabetically
        let mut keys: Vec<&String> = metadata.keys().collect();
        keys.sort();
        keys.iter().map(|k| (*k).clone()).collect()
    };

    for (i, media_id) in ordered_ids.iter().enumerate() {
        let info = match metadata.get(media_id) {
            Some(v) => v,
            None => continue,
        };

        let kind = info.get("e").and_then(|e| e.as_str()).unwrap_or("");

        match kind {
            "Image" => {
                let url = match info["s"]["u"].as_str() {
                    Some(u) => u,
                    None => continue,
                };
                let url = url.replace("&amp;", "&");

                let mime = info.get("m").and_then(|m| m.as_str()).unwrap_or("image/jpg");
                let ext = mime.rsplit('/').next().unwrap_or("jpg");

                let filename = format!("{}_{:02}.{}", post_id, i + 1, ext);
                let dest = dir.join(&filename);

                if dest.exists() {
                    continue;
                }

                match download_file(&url, &dest) {
                    Ok(()) => println!("downloaded: {} (gallery image)", dest.display()),
                    Err(e) => {
                        eprintln!("warning: gallery image download failed for {}: {}", media_id, e)
                    }
                }
            }
            "RedditVideo" => {
                // Try fallbackUrl first, then dashUrl, then hlsUrl
                let vurl = info
                    .get("fallbackUrl")
                    .and_then(|u| u.as_str())
                    .or_else(|| info.get("dashUrl").and_then(|u| u.as_str()))
                    .or_else(|| info.get("hlsUrl").and_then(|u| u.as_str()));

                if let Some(vurl) = vurl {
                    let clean = vurl.split('?').next().unwrap_or(vurl);
                    let filename = format!("{}_{:02}.mp4", post_id, i + 1);
                    let dest = dir.join(&filename);

                    if dest.exists() {
                        continue;
                    }

                    match download_file(&clean, &dest) {
                        Ok(()) => println!("downloaded: {} (gallery video)", dest.display()),
                        Err(e) => {
                            eprintln!(
                                "warning: gallery video download failed for {}: {}",
                                media_id, e
                            )
                        }
                    }
                } else {
                    eprintln!(
                        "warning: no video URL found for gallery media {} (post {})",
                        media_id, post_id
                    );
                }
            }
            _ => {
                eprintln!(
                    "warning: unknown gallery media type '{}' for media {} (post {})",
                    kind, media_id, post_id
                );
            }
        }
    }
}

fn download_media_for_post(post_data: &Value, dir: &Path) {
    let id = post_data
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("post");

    // Handle gallery posts
    if post_data
        .get("is_gallery")
        .and_then(|g| g.as_bool())
        .unwrap_or(false)
    {
        download_gallery(post_data, dir, id);
        return;
    }

    let (url, kind) = match find_media_url(post_data) {
        Some(m) => m,
        None => return,
    };

    if kind != "image" && kind != "video" {
        return;
    }

    let ext = match kind {
        "image" => url.rsplit('.').next().unwrap_or("jpg").to_string(),
        _ => "mp4".to_string(),
    };

    let filename = format!("{}_{}.{}", id, kind, ext);
    let dest = dir.join(&filename);

    if dest.exists() {
        return;
    }

    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("warning: cannot create directory: {}", e);
        return;
    }

    match download_file(&url, &dest) {
        Ok(()) => println!("downloaded: {} ({})", dest.display(), kind),
        Err(e) => eprintln!("warning: download failed for {}: {}", id, e),
    }
}

fn download_all(v: &Value, cli: &Cli) {
    let dir = Path::new(&cli.dir);

    let children: Vec<&Value> = v
        .get("data")
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(|c| c.get("data")).collect())
        .unwrap_or_default();

    if !children.is_empty() {
        for post_data in children {
            download_media_for_post(post_data, dir);
        }
        return;
    }

    if let Some(arr) = v.as_array() {
        if let Some(first) = arr.first() {
            let children: Vec<&Value> = first
                .get("data")
                .and_then(|d| d.get("children"))
                .and_then(|c| c.as_array())
                .map(|arr| arr.iter().filter_map(|c| c.get("data")).collect())
                .unwrap_or_default();
            for post_data in &children {
                download_media_for_post(post_data, dir);
            }
            if !children.is_empty() {
                return;
            }
        }
    }

    eprintln!("warning: no posts found to download");
}

fn run(cli: &Cli, limit: u32, pg: u32) -> Result<Value, String> {
    // Post detail: single fetch, ignore pagination
    if cli.post.is_some() {
        let url = build_url(cli, limit, None, None);
        return fetch(&url);
    }

    // Single-fetch mode with explicit --before (only when pg == 1)
    if cli.before.is_some() && pg == 1 && cli.after.is_none() {
        let url = build_url(cli, limit, None, cli.before.as_deref());
        return fetch(&url);
    }

    // Walk pages using the after cursor
    let mut after = cli.after.clone();
    let mut last: Option<Value> = None;

    for i in 0..pg {
        let url = build_url(cli, limit, after.as_deref(), None);
        let v = fetch(&url)?;
        after = extract_after(&v);
        last = Some(v);
        if after.is_none() && i + 1 < pg {
            break; // no more pages
        }
    }
    Ok(last.expect("at least one fetch must have happened"))
}

fn main() {
    let cli = Cli::parse();
    let limit = cli.limit.clamp(1, 100);
    let pg = if cli.pg == 0 { 1 } else { cli.pg };

    match run(&cli, limit, pg) {
        Ok(v) => {
            if cli.download {
                download_all(&v, &cli);
            } else {
                print_output(&v, &cli);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
