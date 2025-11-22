use crate::prelude::*;

pub async fn discovery(domain: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let domain = Domain::new(&domain)?.save(&ctx).await?;

    // Create Node from current host
    let mut node = Node::from_current_host(&domain, &ctx).await?;

    // Collect host information
    node.collect_host_info(&ctx).await?;

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

    // IP address - create Ip entity and store its hash
    let ip_hash = if let Ok(local_ip) = local_ip_address::local_ip() {
        let ip_str = local_ip.to_string();
        let ip = Ip::create(&ip_str, &node.hash, &ctx).await?;
        node.ip = Some(ip.hash.clone());
        Some(ip.hash)
    } else {
        None
    };

    // Save updated Node
    let mut entity: Entity = node.into();

    if let Some(hash) = ip_hash {
        entity.add_child(&hash);
    }

    ctx.storage().put(&entity).await?;

    // Save index (including environment index)
    let node: Node = entity.try_into()?;
    node.index()?.save(&ctx).await?;

    println!("✅ Discovery completed successfully");
    println!();
    println!("{}", node);

    Ok(())
}

pub async fn list() -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let entities = ctx.storage().list().await?;

    Entity::view(entities);

    Ok(())
}

pub async fn delete(pattern: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let mut entities = ctx.storage().find(&pattern).await?;
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
    let entities = ctx.storage().find(&pattern).await?;
    for entity in entities {
        println!("{}", entity)
    }

    Ok(())
}
