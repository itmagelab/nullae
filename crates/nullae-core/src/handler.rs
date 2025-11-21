use crate::prelude::*;

pub async fn discovery(domain: String) -> Result<(), anyhow::Error> {
    let repository = Repository::new()?;
    let domain = Domain::new(&domain)?.save(&repository).await?;
    let node = Node::from_current_host(&domain, &repository).await?;
    node.index()?.save(&repository).await?;

    Ok(())
}

pub async fn list() -> Result<(), anyhow::Error> {
    let repository = Repository::new()?;
    let entities = repository.list().await?;

    Entity::view(entities);

    Ok(())
}

pub async fn delete(pattern: String) -> Result<(), anyhow::Error> {
    let repository = Repository::new()?;
    let mut entities = repository.find(&pattern).await?;
    let same = if let Some(first) = entities.first() {
        entities.iter().all(|e| e.hash() == first.hash())
    } else {
        true
    };
    if let Some(entity) = entities.pop()
        && same
    {
        entity.delete(&repository).await?;
    };

    Ok(())
}

pub async fn show(pattern: String) -> Result<(), anyhow::Error> {
    let repository = Repository::new()?;
    let entities = repository.find(&pattern).await?;
    for entity in entities {
        println!("{}", entity)
    }

    Ok(())
}
