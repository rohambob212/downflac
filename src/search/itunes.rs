use crate::search::structs::Music;
use reqwest::Client as httpclient;
pub async fn isearch(
    clnt: httpclient,
    name: String,
) -> Result<Vec<Music>, Box<dyn std::error::Error>> {
    let result: serde_json::Value = clnt
        .get("https://itunes.apple.com/search")
        .query(&[
            ("term", name),
            ("entity", "song".to_string()),
            ("limit", "5".to_string()),
        ])
        .send()
        .await
        .expect("There was an error searching your query in deezer")
        .json()
        .await
        .expect("there was an error parsing search JSON");
    let musics = result["results"].as_array().expect("error");
    let mut mames: Vec<Music> = Vec::new();
    for music in musics {
        let title = music["trackName"].to_string().trim_matches('"').to_string();
        let artist = music["artistName"]
            .to_string()
            .trim_matches('"')
            .to_string();
        mames.push(Music { title, artist });
    }
    Ok(mames)
}
