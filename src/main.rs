mod api;
mod search {
    pub mod deezer;
    pub mod itunes;
    pub mod music;
}
use std::{collections::HashMap, env};
mod download;
mod share;
use reqwest::Client as httpclient;

const HELP: &str = "\
downflac - search and download music written purely in rust

USAGE:
    downflac [OPTIONS] [SONG NAME...]

ARGS:
    <SONG NAME>...    Song to search for. All arguments are joined with
                      spaces and used as the search query.
                      If omitted, you are prompted to enter a name.

OPTIONS:
    -h, --help        Print this help and exit

EXAMPLES:
    downflac never gonna give you up
    downflac

PIPELINE:
    1. Matches up to 5 songs on Deezer for your query.
    2. Searches Kuwo for each match.
    3. Prompts you to select a result.
    4. Downloads it to '<title> by <artist> (<album>).<format>'.

SHARING:
    After selecting a song you can create a share code instead of
    downloading. Paste that code as the search query to download the
    exact same file. Nothing leaves your machine.

PAUSING:
    While a download runs, press Enter to pause it and Enter again
    to resume.
";

const ILLEGAL: &str = if cfg!(windows) {
    "<>:\"/\\|?*"
} else if cfg!(target_os = "macos") {
    "/:"
} else {
    "/"
};

fn has_illegal(s: &str) -> bool {
    s.chars().any(|c| ILLEGAL.contains(c) || c.is_control())
}

fn closest(c: char) -> char {
    match c {
        '<' => '＜',
        '>' => '＞',
        ':' => '：',
        '"' => '＂',
        '/' => '／',
        '\\' => '＼',
        '|' => '｜',
        '?' => '？',
        '*' => '＊',
        c if c.is_control() => '_',
        c => c,
    }
}

fn autofix(s: &str) -> String {
    s.chars()
        .map(closest)
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_string()
}

fn resolve_name(choice: &str, fmt: &str) -> Option<String> {
    if !has_illegal(choice) {
        return Some(format!("{choice}.{fmt}"));
    }
    let mut stem = autofix(choice);
    if stem.is_empty() {
        stem = "download".to_string();
    }
    let auto = format!("{stem}.{fmt}");
    let keep = format!("save as \"{auto}\"");
    let typeit = "enter a file name".to_string();
    let cancel = "cancel download".to_string();
    println!("\"{choice}\" cannot be used as a file name on this system");
    loop {
        let opts = vec![keep.clone(), typeit.clone(), cancel.clone()];
        match inquire::Select::new("how should the file be named?", opts).prompt() {
            Ok(o) if o == keep => return Some(auto),
            Ok(o) if o == typeit => {
                match inquire::Text::new("file name:")
                    .with_default(&auto)
                    .prompt()
                {
                    Ok(nm) => {
                        let nm = nm.trim().trim_end_matches(['.', ' ']).to_string();
                        if !nm.is_empty() && !has_illegal(&nm) {
                            let ext = format!(".{fmt}");
                            if nm.to_lowercase().ends_with(&ext) {
                                return Some(nm);
                            }
                            return Some(format!("{nm}{ext}"));
                        }
                        println!("\"{nm}\" cannot be used as a file name on this system");
                    }
                    Err(_) => return None,
                }
            }
            Ok(o) if o == cancel => return None,
            Ok(_) => unreachable!(),
            Err(_) => return None,
        }
    }
}

async fn save(htc: httpclient, dt: api::Mdata, choice: String) {
    let pth = match resolve_name(&choice, &dt.minfo.format) {
        Some(pth) => pth,
        None => {
            println!("Download Cancelled");
            return;
        }
    };
    let link: String = match api::getmusic(htc.clone(), dt.clone()).await {
        Ok(l) => l,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };
    let down: Result<(), Box<dyn std::error::Error>> = download::download(link, pth.clone());
    match down {
        Ok(_) => println!("Download Completed"),
        Err(e) => println!("Error: {}", e),
    }
    if dt.pic.is_empty() {
        println!("No cover to embed");
        return;
    }
    println!("{}", dt.pic);
    let embed: Result<(), Box<dyn std::error::Error>> = download::embed_cover(pth, dt.pic);
    match embed {
        Ok(_) => println!("Embed Completed"),
        Err(e) => println!("Error: {}", e),
    }
}

async fn act(htc: httpclient, dt: api::Mdata, choice: String) {
    let opts: Vec<String> = vec!["download".to_string(), "get a share code".to_string()];
    match inquire::Select::new("what do you want to do?", opts).prompt() {
        Ok(o) if o == "download" => save(htc, dt, choice).await,
        Ok(_) => match share::create(&choice, &dt) {
            Some(code) => println!("Share code:\n{code}"),
            None => println!("Error: could not build a share code"),
        },
        Err(e) => println!("there was an error: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let roots = webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .filter_map(|c| reqwest::Certificate::from_der(c).ok());
    let mayhtc: Result<httpclient, reqwest::Error> =
        httpclient::builder().tls_certs_only(roots).build();
    let htc = match mayhtc {
        Ok(clnt) => clnt,
        Err(e) => {
            eprintln!("There was an error building the http client: {e}");
            return Ok(());
        }
    };
    let argvs: Vec<String> = env::args().collect();
    if argvs.len() > 1 && matches!(argvs[1].as_str(), "-h" | "--help" | "help") {
        print!("{HELP}");
        return Ok(());
    }
    let mut name: String = String::new();
    if argvs.len() > 1 {
        name = argvs[1..].to_vec().join(" ");
    } else {
        let asname = inquire::Text::new("Enter the name of your intended song:").prompt();
        match asname {
            Ok(nm) => name = nm,
            Err(e) => println!("An error Happend: {}", e),
        }
    }
    if let Some(sid) = share::parse(&name) {
        match share::resolve(&sid) {
            Some(shared) => {
                let choice = format!("{} by {} ({})", shared.title, shared.artist, shared.album);
                save(htc.clone(), shared.data, choice).await;
            }
            None => println!("Error: that is not a valid share code"),
        }
        return Ok(());
    }
    let mut opts: Vec<String> = Vec::new();

    let mut mhash: HashMap<String, api::Mdata> = HashMap::new();
    let nusics: Vec<api::Nusic> = api::getmdata(htc.clone(), name)
        .await
        .expect("there was an error searching for your music");
    for nsc in nusics {
        let nm: String = format!("{} by {} ({})", nsc.title, nsc.artist, nsc.album);
        mhash.insert(nm.clone(), nsc.data);
        opts.push(nm);
    }
    let nmsc: Result<String, inquire::InquireError> =
        inquire::Select::new("select a song", opts).prompt();
    match nmsc {
        Ok(choice) => {
            let dt: api::Mdata = mhash[&choice].clone();
            act(htc.clone(), dt, choice).await;
        }
        Err(e) => println!("there was an error: {}", e),
    }
    Ok(())
}
