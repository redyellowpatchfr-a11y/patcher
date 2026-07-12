# Zénith Patcher — Undertale Yellow & Red and Yellow FR

Patcher de traduction française pour **Undertale Yellow** et **Undertale Red & Yellow**.

> Développé par la team Zénith (Cronos, Anthan)

---

## 📦 Contenu du dépôt

| Fichier | Description |
|---|---|
| `versions.json` | Métadonnées des versions (lu automatiquement par le patcher) |
| Releases → `uty-fr-v*.xdelta` | Patch xdelta Undertale Yellow FR |
| Releases → `ry-fr-v*.xdelta` | Patch xdelta Red & Yellow FR (vanilla → R&Y FR) |
| Releases → `zenith-patcher-linux` | Patcher GUI Linux |
| Releases → `zenith-patcher-windows.exe` | Patcher GUI Windows |

---

## 🎮 Comment utiliser le patcher

### Option A — Depuis Undertale vanilla (recommandé pour R&Y)

Le patch `ry-fr-v2.2.0.xdelta` transforme **directement** votre `game.unx` / `data.win` Undertale vanilla en Red & Yellow traduit en français.

1. Avoir Undertale installé (Steam, GOG ou autre)
2. Lancer `zenith-patcher-linux` ou `zenith-patcher-windows.exe`
3. Sélectionner "Red & Yellow" → "Option A : J'ai déjà le jeu"
4. Le patcher détecte automatiquement Undertale ou vous demande le chemin
5. Applique le patch xdelta → R&Y FR installé ✓

### Option B — Depuis itch.io (Undertale Yellow uniquement)

Pour Undertale Yellow, le patcher peut télécharger le repack complet depuis itch.io.

---

## 🛠️ Fichiers xdelta

### `ry-fr-v2.2.0.xdelta`
- **Source** : `game.unx` / `data.win` Undertale vanilla (v1.08)
- **Cible** : Red & Yellow FR v2.2
- **Taille** : ~36 Mo
- **SHA256** : `a606f3445a5a0100371d55d0ff66c2f2e757691948337097834e26e0af3b03a3`

### `uty-fr-v0.5.0.xdelta`  
- **Source** : `data.win` Undertale Yellow vanilla
- **Cible** : Undertale Yellow FR v0.5
- **Taille** : ~54 Mo
- **SHA256** : `c227defe6d78994e3f1b9323a0c462a91906049bbf381930ca6964eeda23dfa3`

---

## 📋 Format `versions.json`

Le patcher lit ce fichier pour vérifier les mises à jour disponibles :

```json
{
  "projects": {
    "ry-fr": {
      "version": "2.2.0",
      "patch_url": "URL directe vers le .xdelta",
      "date": "2026-07-12",
      "changelog": "Description des changements",
      "patch": {
        "filename": "ry-fr-v2.2.0.xdelta",
        "sha256": "checksum de vérification",
        "size": 38093797
      }
    }
  }
}
```

---

## 🔗 Liens

- [Discord Zénith](https://discord.gg/mAwZBxhSSf)
- [Site web Zénith](https://zenith.fr)
- [Undertale Yellow (itch.io)](https://taediumbreak.itch.io/undertale-yellow)
