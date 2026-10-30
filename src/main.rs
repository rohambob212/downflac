mod api;
mod search {
    pub mod deezer;
}
use std::{collections::HashMap, env};
mod download;
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
                        let nm = nm.trim().to_string();
                        if !nm.is_empty() && !has_illegal(&nm) {
                            return Some(nm);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_platform_illegal_chars() {
        assert!(!has_illegal("Song by Artist (Album)"));
        assert!(has_illegal("AC/DC by AC\\DC (Live: 1979)"));
        assert!(has_illegal("bad\nname"));
        if cfg!(windows) {
            assert!(has_illegal("what? by who* (x)"));
        }
    }

    #[test]
    fn autofix_yields_saveable_names() {
        assert!(!has_illegal(&autofix("AC/DC by AC\\DC (Live: 1979)")));
        assert!(!has_illegal(&autofix("trailing dots... ")));
        assert_eq!(autofix("AC/DC"), "AC／DC");
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
    let mut opts: Vec<String> = Vec::new();

    let mut mhash: HashMap<String, api::Mdata> = HashMap::new();
    let nusics: Vec<api::Nusic> = api::getmdata(htc.clone(), name)
        .await
        .expect("there was an error searching for your music");
    // let nusics: Vec<deezer::Music> = deezer::dsearch(htc.clone(), name)
    //     .await
    //     .expect("there was an error searching for your music");
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
            match resolve_name(&choice, &dt.minfo.format) {
                Some(pth) => {
                    let link: String = api::getmusic(htc.clone(), dt.clone()).await?;
                    let down: Result<(), Box<dyn std::error::Error>> =
                        download::download(link, pth.clone());
                    match down {
                        Ok(_) => println!("Download Completed"),
                        Err(e) => println!("Error: {}", e),
                    }
                    println!("{}", dt.pic);
                    let embed: Result<(), Box<dyn std::error::Error>> =
                        download::embed_cover(pth, dt.pic);
                    match embed {
                        Ok(_) => println!("Embed Completed"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                None => println!("Download Cancelled"),
            }
        }
        Err(e) => println!("there was an error: {}", e),
    }
    Ok(())
}
