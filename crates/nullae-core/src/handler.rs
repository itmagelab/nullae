use crate::prelude::*;

pub async fn discovery(domain: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let domain = Domain::new(&domain)?.save(&ctx).await?;
    let node = Node::from_current_host(&domain, &ctx).await?;
    node.index()?.save(&ctx).await?;

    Ok(())
}

pub async fn list() -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let entities = ctx.repository().list().await?;

    Entity::view(entities);

    Ok(())
}

pub async fn delete(pattern: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let mut entities = ctx.repository().find(&pattern).await?;
    let same = if let Some(first) = entities.first() {
        entities.iter().all(|e| e.hash() == first.hash())
    } else {
        true
    };
    if let Some(entity) = entities.pop()
        && same
    {
        entity.delete(&ctx).await?;
    };

    Ok(())
}

pub async fn show(pattern: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let entities = ctx.repository().find(&pattern).await?;
    for entity in entities {
        println!("{}", entity)
    }

    Ok(())
}
