# Sandbox management recipe
sandbox action="help":
    @if [ "{{action}}" = "help" ]; then \
        printf "\033[1;36m====================================================\033[0m\n"; \
        printf "\033[1;36m             0AE SANDBOX ORCHESTRATION             \033[0m\n"; \
        printf "\033[1;36m====================================================\033[0m\n"; \
        printf "Usage: just sandbox <action>\n\n"; \
        printf "Available actions:\n"; \
        printf "  \033[32mup\033[0m         Start main Consul/Caddy and build/start sandbox nodes\n"; \
        printf "  \033[32mdown\033[0m       Tear down both sandbox nodes and main Consul services\n"; \
        printf "  \033[32mstatus\033[0m     Show status of all sandbox and main containers\n"; \
        printf "  \033[32mdiscovery\033[0m  Run 0ae discovery sequentially across node1, node2, node3\n"; \
        printf "  \033[32mlist\033[0m       Query 0ae list from inside node1 to inspect the cluster\n\n"; \
        exit 0; \
    elif [ "{{action}}" = "up" ]; then \
        printf "🚀 Starting main Consul cluster and Caddy load balancer...\n"; \
        docker compose up -d; \
        printf "🚀 Starting sandbox nodes...\n"; \
        docker compose -f compose.sandbox.yaml up -d --build; \
    elif [ "{{action}}" = "down" ]; then \
        printf "🛑 Tearing down sandbox nodes...\n"; \
        docker compose -f compose.sandbox.yaml down -v; \
        printf "🛑 Tearing down main Consul cluster...\n"; \
        docker compose down -v; \
    elif [ "{{action}}" = "status" ]; then \
        printf "📊 Main and Sandbox Containers Status:\n"; \
        docker compose ps; \
        docker compose -f compose.sandbox.yaml ps; \
    elif [ "{{action}}" = "discovery" ]; then \
        printf "🔎 Simulating infrastructure auto-discovery...\n"; \
        for node in node1 node2 node3; do \
            printf "\n\033[1;33m>>> Running '0ae discovery' inside %s...\033[0m\n" "$node"; \
            docker compose -f compose.sandbox.yaml exec $node /bin/0ae discovery; \
        done; \
    elif [ "{{action}}" = "list" ]; then \
        printf "📋 Querying unified registry from node1...\n"; \
        docker compose -f compose.sandbox.yaml exec node1 /bin/0ae list -k node; \
    else \
        printf "\033[1;31m❌ Unknown action: %s\033[0m\n" "{{action}}"; \
        printf "Run 'just sandbox' to see the list of available actions.\n"; \
        exit 1; \
    fi
