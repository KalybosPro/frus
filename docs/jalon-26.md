# Jalon 26 — Subscriptions (sources continues de messages)

La dernière grande pièce du modèle Elm : le pendant **flux** de `Command`
(ponctuel). L'app déclare des **sources continues** de messages selon son état ;
le framework les **démarre/arrête** par diff.

## API

```rust
fn subscription(&self) -> Subscription<Msg> { Subscription::none() }  // ajout au trait

Subscription::none()
Subscription::batch([...])
Subscription::every(Duration, |instant| Msg)   // un message par intervalle
```

Chaque souscription porte un **id** stable = hash de sa recette (type + durée).
Deux `every(1s)` = **une seule** souscription.

## Fonctionnement (framework)

- `sync_subscriptions()` est appelé **au démarrage** et après **chaque**
  `dispatch` : il compare les ids déclarés à ceux en cours
  (`HashMap<u64, Sender<()>>`) → démarre les nouveaux, **annule** les disparus.
- Un `every` = un thread bouclant `rx.recv_timeout(interval)` :
  - **Timeout** → `proxy.send_event(make(now))` (le message revient dans la boucle
    via `user_event`, comme un résultat de `Command`) ;
  - **Sender droppé** (annulation) ou boucle fermée → le thread sort.
- Annuler = retirer le `Sender` de la table (drop) ; le thread sort à son prochain
  réveil. Symétrie totale avec `Command` (même proxy, mêmes threads).

## Décisions techniques (alternatives)

- **Thread par souscription** (cohérent avec `Command`) vs *timer wheel* unique →
  thread par sub, simple et suffisant.
- **Diff par hash de recette** (façon Elm) → deux `every` identiques fusionnent ;
  la souscription persiste tant qu'elle est redéclarée à l'identique.

## Démo — chrono

En-tête : « · Ns » (secondes écoulées) + bouton **Pause/Reprendre**. `running`
pilote la souscription : `running ? every(1s, |_| Tick) : none()`. Basculer
démontre le **démarrage/arrêt** effectif du thread par le diff. `init()` démarre
le chrono ; `Msg::Tick` incrémente le compteur.

## Tests

- `Subscription` : `none`/`is_empty`, `every` → id stable par durée (même durée =
  même id, durées différentes = ids différents), `batch` combine.
- Démo : `subscription()` vide en pause, non vide sinon (id stable entre deux
  évaluations) ; `Tick` incrémente le compteur.
- **Bout-en-bout** : la démo tourne 6 s → **5 ticks** observés dans les logs
  (1s→5s), preuve que le flux `every → proxy → user_event → update` fonctionne.
- Totaux : 3 frus-shell (subscription) + 9 frus-demo.

## Limites (v1)

- Un thread par souscription (pas de pool) ; latence d'annulation ≤ un intervalle.
- Seul `every` pour l'instant (le mécanisme accueillera `Kind` supplémentaires :
  clavier global, événements fenêtre, flux externes).
- Horloge murale formatée nécessiterait une lib de temps → chrono **écoulé**.
