use crate::search::music::Music;
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
