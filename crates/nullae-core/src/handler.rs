use crate::prelude::*;

pub async fn auto(domain: String) -> Result<(), anyhow::Error> {
    let repository = Repository::new().unwrap();
    let domain = Domain::new(&domain).save(&repository).await?;
    let node = Node::from_current_host(&domain, &repository).await?;
    node.index().unwrap().save(&repository).await.unwrap();

    Ok(())
}

pub async fn list() -> Result<(), anyhow::Error> {
    let repository = Repository::new().unwrap();
    let entities = repository.list().await?;

    Entity::view(entities);

    Ok(())
}

pub async fn delete(pattern: String) -> Result<(), anyhow::Error> {
    let repository = Repository::new().unwrap();
    let mut entities = repository.find(&pattern).await?;
    if entities.len() > 1 {
        Entity::view(entities);
        anyhow::bail!("Too many entities");
    }
    if let Some(entity) = entities.pop() {
        entity.delete(&repository).await?;
    };

    Ok(())
}

pub async fn show(hash: String) -> Result<(), anyhow::Error> {
    let repository = Repository::new().unwrap();
    let entities = repository.find(&hash).await?;
    for entity in entities {
        println!("{}", entity)
    }

    Ok(())
}
