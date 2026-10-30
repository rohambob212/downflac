use crate::search::itunes::isearch;
use reqwest::Client as httpclient;
use serde::Serialize;
use serde_json::Value;
#[derive(Serialize)]
struct Search {
    platform: String,
    keyword: String,
    page: u64,
    size: u64,
}

#[derive(Serialize)]
struct Udata {
    platform: String,
    songid: String,
    format: String,
    bitrate: String,
    time: String,
    sign: String,
}
#[derive(serde::Deserialize, Serialize, Debug, Clone)]
pub struct Minfobj {
    pub format: String,
    pub bitrate: String,
    pub level: String,
    pub size: String,
}

pub struct Nusic {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub data: Mdata,
}

#[derive(Clone)]
pub struct Mdata {
    pub id: String,
    pub time: String,
    pub sign: String,
    pub minfo: Minfobj,
    pub pic: String,
}

pub async fn getmdata(
    clnt: httpclient,
    name: String,
) -> Result<Vec<Nusic>, Box<dyn std::error::Error>> {
    let mut nams: Vec<Nusic> = Vec::new();
    let dsrches: Vec<crate::search::music::Music> = isearch(clnt.clone(), name.clone()).await?;
    for dsrch in dsrches {
        let srch: Search = Search {
            platform: "kuwo".to_string(),
            keyword: format!("{} {}", dsrch.title.clone(), dsrch.artist.clone(),),
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
    }
    Ok(nams)
}

pub async fn getmusic(clnt: httpclient, data: Mdata) -> Result<String, Box<dyn std::error::Error>> {
    let udata: Udata = Udata {
        platform: "kuwo".to_string(),
        songid: data.id,
        format: data.minfo.format,
        bitrate: data.minfo.bitrate,
        time: data.time,
        sign: data.sign,
    };
    let linkdata: serde_json::Value = clnt
        .post("https://flac.music.hi.cn/ajax.php?act=getUrl")
        .form(&udata)
        .send()
        .await
        .expect("there was en error getting the url")
        .json()
        .await
        .expect("the get url response was not valid json");
    Ok(linkdata["data"]["url"]
        .to_string()
        .trim_matches('"')
        .to_string())
}
