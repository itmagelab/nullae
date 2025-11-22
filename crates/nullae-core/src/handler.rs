use crate::prelude::*;

pub async fn discovery(domain: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let domain = Domain::new(&domain)?.save(&ctx).await?;

    // Create Node from current host
    let mut node = Node::from_current_host(&domain, &ctx).await?;

    // Collect host information
    node.collect_host_info()?;

    // Read environment from env var (optional)
    if let Ok(env) = std::env::var("NULLAE_ENVIRONMENT") {
        node = node.with_environment(&env);
    }

    // Read tags from env var (optional)
    if let Ok(tags_str) = std::env::var("NULLAE_TAGS") {
        let tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !tags.is_empty() {
            node = node.with_tags(tags);
        }
    }

    // Save updated Node
    let entity: Entity = node.into();
    ctx.repository().put(&entity).await?;

    // Save index (including environment index)
    let node: Node = entity.try_into()?;
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
