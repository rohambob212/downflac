use reqwest::Client as httpclient;

use crate::search::structs::Music;
pub async fn dsearch(
    clnt: httpclient,
    name: String,
) -> Result<Vec<Music>, Box<dyn std::error::Error>> {
    let result: serde_json::Value = clnt
        .get("https://api.deezer.com/search")
        .query(&[("q", name), ("limit", "5".to_string())])
        .send()
        .await
        .expect("There was an error searching your query in deezer")
        .json()
        .await
        .expect("there was an error parsing search JSON");
    let musics = result["data"]
        .as_array()
        .expect("deezer didn't return any results");
    let mut mames: Vec<Music> = Vec::new();
    for music in musics {
        let title = music["title"].to_string().trim_matches('"').to_string();
        let artist = music["artist"]["name"]
            .to_string()
            .trim_matches('"')
            .to_string();
        mames.push(Music { title, artist });
    }
    Ok(mames)
}
