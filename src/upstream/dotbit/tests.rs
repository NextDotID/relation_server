use crate::graph::edge::Hold;
use crate::upstream::Target;
use crate::{error::Error, upstream::dotbit::DotBit, upstream::Fetcher};
use crate::{
    graph::new_db_connection, graph::vertex::Identity, upstream::Platform, util::naive_now,
};

#[tokio::test]
async fn test_smoke_dotbit_by_dotbit_identity() -> Result<(), Error> {
    let target = Target::Identity(Platform::Dotbit, "test0920.bit".into());

    DotBit::fetch(&target).await?;

    let db = new_db_connection().await?;
    let found = Identity::find_by_platform_identity(&db, &target.platform()?, &target.identity()?)
        .await?
        .expect("Record not found");

    assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

    Ok(())
}

#[tokio::test]
async fn test_smoke_dotbit_reverse_record() -> Result<(), Error> {
    
    //0x9176acd39a3a9ae99dcb3922757f8af4f94cdf3c => justing.bit
    let target = Target::Identity(
        Platform::Ethereum,
        "0x3a6cab3323833f53754db4202f5741756c436ede".into(),
    );

    DotBit::fetch(&target).await?;

    let db = new_db_connection().await?;
   
    Identity::find_by_platform_identity(&db, &target.platform()?, &target.identity()?)
            .await?
            .expect("Record not found");
    
    Identity::find_by_platform_identity(&db, &Platform::Dotbit, "justing.bit")
        .await?
        .expect("Record not found");

    Ok(())
}
