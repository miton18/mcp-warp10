# Installation du serveur MCP Warp10 dans Claude

## Prérequis

1. Node.js installé sur votre système
2. Le serveur MCP Warp10 compilé (`npm run build`)
3. Un serveur Warp10 accessible avec un token valide

## Configuration dans Claude Desktop

### Option 1 : Avec le script de lancement (recommandé)

Ajoutez cette configuration dans votre fichier `~/.claude.json` dans la section `mcpServers` :

```json
"warp10": {
  "command": "/home/miton/code/nodejs/mcp-warp10/launch.sh",
  "args": [],
  "env": {
    "WARP10_ENDPOINT": "http://localhost:8080/api/v0",
    "WARP10_TOKEN": "VOTRE_TOKEN_WARP10"
  }
}
```

### Option 2 : Avec Node.js directement

```json
"warp10": {
  "command": "node",
  "args": ["/home/miton/code/nodejs/mcp-warp10/dist/index.js"],
  "env": {
    "WARP10_ENDPOINT": "http://localhost:8080/api/v0",
    "WARP10_TOKEN": "VOTRE_TOKEN_WARP10"
  }
}
```

### Option 3 : Avec NPX (si publié sur npm)

```json
"warp10": {
  "command": "npx",
  "args": ["-y", "@mcp/warp10"],
  "env": {
    "WARP10_ENDPOINT": "http://localhost:8080/api/v0",
    "WARP10_TOKEN": "VOTRE_TOKEN_WARP10"
  }
}
```

## Variables d'environnement

- `WARP10_ENDPOINT` : URL de votre serveur Warp10 (par défaut : `http://localhost:8080/api/v0`)
- `WARP10_TOKEN` : Token d'authentification pour Warp10
- `WARP10_READ_TOKEN` (optionnel) : Token en lecture seule
- `WARP10_WRITE_TOKEN` (optionnel) : Token en écriture seule

## Vérification de l'installation

1. Redémarrez Claude Desktop après avoir modifié `~/.claude.json`
2. Le serveur MCP Warp10 devrait apparaître dans les serveurs disponibles
3. Testez avec une commande simple :
   ```
   Utilise l'outil warp10_find pour lister toutes les séries temporelles disponibles
   ```

## Dépannage

### Le serveur ne se connecte pas

1. Vérifiez que le chemin vers le script est correct
2. Assurez-vous que le script `launch.sh` est exécutable :
   ```bash
   chmod +x /home/miton/code/nodejs/mcp-warp10/launch.sh
   ```
3. Vérifiez que le projet est compilé :
   ```bash
   cd /home/miton/code/nodejs/mcp-warp10
   npm run build
   ```

### Erreurs d'authentification

1. Vérifiez que votre token Warp10 est valide
2. Testez la connexion directement :
   ```bash
   curl -H "X-Warp10-Token: VOTRE_TOKEN" http://localhost:8080/api/v0/exec -d "IDENT"
   ```

### Logs et débogage

Les logs du serveur MCP peuvent être consultés dans les logs de Claude Desktop.

## Support

Pour toute question ou problème, consultez le README principal ou ouvrez une issue sur le dépôt GitHub.