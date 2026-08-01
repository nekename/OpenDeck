# Teams Contact (OpenDeck / OpenAction)

Plugin importable : cherche un contact Teams/Graph, affiche sa photo sur la touche,
**simple clic = chat**, **double clic = appel audio**.

## Installation

1. Copie le dossier `com.contactretriever.teams.sdPlugin` dans le repertoire plugins d'OpenDeck :
   - Linux : `~/.config/opendeck/plugins/`
   - macOS : `~/Library/Application Support/opendeck/plugins/`
   - Flatpak : `~/.var/app/me.amankhanna.opendeck/config/opendeck/plugins/`
2. Redemarre OpenDeck (ou reactive le plugin).
3. **Node.js >= 20** requis (WebSocket natif : Node 22+ recommande ; sinon `npm install` dans le dossier).

Aucune compilation. Dossier pret a l'emploi.

## Usage

1. Glisse l'action **Teams Contact** sur une touche.
2. Dans le property inspector :
   - colle le **token recherche** (Graph ou Substrate)
   - colle le **token photo** Skype (`aud = https://api.spaces.skype.com`)
   - **Enregistrer tokens** (partages entre toutes les touches du plugin)
3. Cherche un nom, clique un resultat → photo assignee a la touche.
4. **Clic** → `msteams://…/l/chat/0/0?users=…`
5. **Double clic** (~400 ms) → `msteams://…/l/call/0/0?users=…` (audio)

## Tokens (~1 h)

| Champ PI | `aud` attendu | Role |
|---|---|---|
| Token recherche | `https://graph.microsoft.com` **ou** `https://outlook.office.com/search` | recherche contacts |
| Token photo | `https://api.spaces.skype.com` | photo via `profilepicturev2` |

Procedure (Teams web, Chrome) :

1. Ouvre https://teams.microsoft.com , `F12` → Network, Preserve log.
2. **Recherche** :
   - Graph : filtre `graph.microsoft.com` → header `Authorization` → JWT apres `Bearer `
   - Substrate (souvent plus simple) : tape un nom dans la barre de recherche Teams → filtre `substrate.office.com` → meme header
3. **Photo** : filtre `profilepicturev2` → Cookies → `authtoken` → JWT entre `Bearer=` et `&origin`
4. Verifie `aud` sur https://jwt.ms

`TEAMS_PART` (defaut `emea-02`) = segment vu dans l'URL `…/api/mt/part/<part>/…`.

## Compatibilite recherche

Le plugin detecte l'audience du token :

- Graph → `/me/people`, puis annuaire `/users` si vide
- Substrate (Powerbar Teams) → suggestions People
- Audience inconnue → tente Graph puis Substrate

Photos : Teams `profilepicturev2` si token Skype present, sinon Graph `/photo/$value`.

## Securite

Les tokens sont des secrets de session. Stockes en clair dans les global settings OpenDeck
(`settings/com.contactretriever.teams.sdPlugin.json`). Ne pas committer. Ils expirent ~1 h.
