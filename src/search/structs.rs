use serde::Serialize;
pub struct Music {
    pub title: String,
    pub artist: String,
}
#[derive(serde::Deserialize, Serialize, Debug, Clone)]
pub struct Minfobj {
    pub format: String,
    pub bitrate: String,
    pub level: String,
    pub size: String,
}
#[derive(Clone)]
pub struct Mdata {
    pub id: String,
    pub time: String,
    pub sign: String,
    pub minfo: Minfobj,
    pub pic: String,
}
pub struct Nusic {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub data: Mdata,
}
