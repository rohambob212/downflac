use crate::api;
use crate::search::structs::{Mdata, Music, Nusic};
use reqwest::Client as httpclient;
use serde_json::Value;
pub async fn spsearch(
    searcher: spotify_anon::AsyncSpotify,
    name: String,
) -> Result<Vec<Music>, Box<dyn std::error::Error>> {
    let trcks: Vec<spotify_anon::TrackHit> = searcher
        .search_tracks(&name, 5)
        .await
        .expect("there was an error searching on spotify");
    let mut retrcks: Vec<Music> = Vec::new();
    for trck in trcks {
        println!("{:?}", trck.cover_url);
        retrcks.push(Music {
            title: trck.name,
            artist: trck.artists.join(" "),
        });
    }
    Ok(retrcks)
}
pub async fn getrack(
    clnt: httpclient,
    searcher: spotify_anon::AsyncSpotify,
    url: String,
) -> Result<Vec<Nusic>, Box<dyn std::error::Error>> {
    let trck: spotify_anon::Track = searcher.track(&url).await?;
    let srch: api::Search = api::Search {
        platform: "kuwo".to_string(),
        keyword: format!("{} {}", trck.name.clone(), trck.artists().join(" ").clone(),),
        page: 1,
        size: 2,
    };
    let nmdata: serde_json::Value = clnt
        .post("https://flac.music.hi.cn/ajax.php?act=search")
        .form(&srch)
        .send()
        .await?
        .json()
        .await?;
    let nms: &serde_json::Value = &nmdata["data"]["list"];
    let mut cid: Value = Value::Null;
    let mut nams: Vec<Nusic> = Vec::new();
    for nm in nms.as_array().expect("Response is wrong") {
        if nm["id"] == cid {
            continue;
        }
        cid = nm["id"].clone();
        nams.push(Nusic {
            title: nm["name"].to_string().trim_matches('"').to_string(),
            artist: nm["artist"].to_string().trim_matches('"').to_string(),
            album: nm["album_name"].to_string().trim_matches('"').to_string(),
            data: Mdata {
                id: nm["id"].to_string().trim_matches('"').to_string(),
                time: nm["time"].to_string().trim_matches('"').to_string(),
                sign: nm["sign"].to_string().trim_matches('"').to_string(),
                minfo: serde_json::from_value(nm["minfo"][0].clone())?,
                pic: nm["pic_url"].to_string().trim_matches('"').to_string(),
            },
        });
    }
    Ok(nams)
}
