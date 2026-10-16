# Analyse fonctionnelle --- Forge

> **Forge** est une application collaborative de gestion de projets et
> de tâches conçue comme l'application de référence pour apprendre à
> construire une application complète avec **Frus**.
>
> **Frontend : Frus + Rust**\
> **Backend : Axum + Rust**\
> **Base de données : PostgreSQL**

------------------------------------------------------------------------

## 1. Vision du produit

Forge permet à une équipe de :

-   créer et gérer des projets ;
-   inviter des membres ;
-   créer, organiser et suivre des tâches ;
-   attribuer des tâches aux membres ;
-   suivre l'avancement des projets ;
-   commenter les tâches ;
-   recevoir des notifications ;
-   consulter l'activité d'un projet ;
-   collaborer en temps réel ;
-   gérer son profil et ses préférences.

Architecture globale :

``` text
                         FORGE
                           │
             ┌─────────────┴─────────────┐
             │                           │
       FRONTEND                      BACKEND
         FRUS                         AXUM
             │                           │
             │ HTTP / WebSocket          │
             └─────────────┬─────────────┘
                           │
                      PostgreSQL
```

------------------------------------------------------------------------

## 2. Objectifs pédagogiques

Le projet doit démontrer les capacités réelles de Frus dans une
application complète.

  Fonctionnalité       Capacité Frus démontrée
  -------------------- ---------------------------------
  Login / Register     Forms, validation, HTTP
  Dashboard            Layout, widgets, state
  Navigation           Router
  Projects             API + CRUD
  Tasks                Lists, forms, interactions
  Task details         Nested navigation
  Search               Input + state + API
  Filters              Reactive state
  Notifications        State + UI
  WebSocket            Temps réel
  Dark mode            Theme
  Responsive UI        Layout
  Loading states       Async state
  Error handling       Network/errors
  Offline              Local storage / synchronisation
  Animations           Animation engine
  Keyboard shortcuts   Input / events
  Tests                Testing framework

Forge doit devenir une **documentation vivante de Frus**.

------------------------------------------------------------------------

# 3. Utilisateurs

## 3.1 Visiteur

Un visiteur non authentifié peut :

-   consulter la page d'accueil ;
-   consulter les informations sur Forge ;
-   créer un compte ;
-   se connecter.

Il ne peut pas accéder aux données privées de l'application.

## 3.2 Utilisateur authentifié

Un utilisateur authentifié peut :

-   consulter son dashboard ;
-   créer et gérer des projets ;
-   rejoindre des projets ;
-   créer et gérer des tâches selon ses permissions ;
-   commenter ;
-   recevoir des notifications ;
-   modifier son profil ;
-   gérer ses préférences.

## 3.3 Rôles dans un projet

Chaque projet possède des membres avec un rôle :

-   **Owner**
-   **Admin**
-   **Member**

Le rôle détermine les actions autorisées.

------------------------------------------------------------------------

# 4. Authentification

## 4.1 Inscription

Formulaire :

``` text
Full name
Email
Password
Confirm password
```

Validation côté frontend :

-   nom obligatoire ;
-   email valide ;
-   mot de passe suffisamment robuste ;
-   confirmation identique.

Endpoint :

``` http
POST /api/v1/auth/register
```

Le backend :

1.  valide les données ;
2.  vérifie que l'email n'existe pas ;
3.  hash le mot de passe ;
4.  crée l'utilisateur ;
5.  initialise ses préférences ;
6.  crée la session ;
7.  retourne les informations nécessaires au frontend.

------------------------------------------------------------------------

## 4.2 Connexion

``` http
POST /api/v1/auth/login
```

Interface :

``` text
┌──────────────────────────────┐
│          Welcome back        │
│                              │
│ Email                        │
│ [________________________]   │
│                              │
│ Password                     │
│ [________________________]   │
│                              │
│          [ Sign in ]         │
│                              │
│ Don't have an account?       │
│ Create account               │
└──────────────────────────────┘
```

------------------------------------------------------------------------

## 4.3 Session

Le frontend doit gérer explicitement :

``` text
Authenticated
Unauthenticated
Loading
Expired session
```

Les routes protégées redirigent automatiquement vers `/login`.

------------------------------------------------------------------------

# 5. Dashboard

Route :

``` text
/app/dashboard
```

Le dashboard affiche :

-   nombre de projets ;
-   tâches assignées ;
-   tâches terminées ;
-   tâches en retard ;
-   projets récents ;
-   activité récente.

Exemple :

``` text
┌──────────────────────────────────────────────────┐
│ Forge                         🔔   Avatar        │
├──────────┬───────────────────────────────────────┤
│          │                                       │
│ Dashboard│  Good morning                       │
│ Projects │                                       │
│ Tasks    │  ┌──────┐ ┌──────┐ ┌──────┐          │
│ Activity │  │  12  │ │  28  │ │   5  │          │
│          │  │Tasks │ │Done  │ │Due   │          │
│ Settings │  └──────┘ └──────┘ └──────┘          │
│          │                                       │
│          │  Recent projects                      │
│          │  ┌────────────────────────────────┐   │
│          │  │ Frus Framework                 │   │
│          │  │ 12 tasks · 4 members           │   │
│          │  └────────────────────────────────┘   │
└──────────┴───────────────────────────────────────┘
```

------------------------------------------------------------------------

# 6. Projets

## 6.1 Liste des projets

Route :

``` text
/app/projects
```

Fonctionnalités :

-   afficher les projets ;
-   rechercher ;
-   filtrer ;
-   créer un projet ;
-   ouvrir un projet ;
-   éventuellement archiver un projet.

------------------------------------------------------------------------

## 6.2 Création d'un projet

Données :

``` text
Project name
Description
Color / Icon
```

Endpoint :

``` http
POST /api/v1/projects
```

------------------------------------------------------------------------

## 6.3 Détail d'un projet

Route :

``` text
/app/projects/:projectId
```

Navigation interne :

``` text
Project
├── Overview
├── Board
├── Tasks
├── Members
├── Activity
└── Settings
```

------------------------------------------------------------------------

# 7. Board

Le Board est l'un des écrans principaux de Forge.

``` text
┌─────────────────────────────────────────────────────────┐
│ Frus Framework                         + New task       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ TODO          IN PROGRESS       REVIEW        DONE      │
│                                                         │
│ ┌─────────┐   ┌─────────┐     ┌─────────┐  ┌────────┐ │
│ │ Task 1  │   │ Task 4  │     │ Task 7  │  │ Task 9 │ │
│ │ #123    │   │ #126    │     │ #129    │  │ #131   │ │
│ └─────────┘   └─────────┘     └─────────┘  └────────┘ │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Statuts :

``` text
TODO
IN_PROGRESS
REVIEW
DONE
```

Une tâche peut être déplacée entre les colonnes.

Le changement est synchronisé avec le backend.

------------------------------------------------------------------------

# 8. Tâches

Une tâche possède notamment :

``` text
Task
├── id
├── title
├── description
├── status
├── priority
├── assignee
├── creator
├── project
├── labels
├── due date
├── createdAt
└── updatedAt
```

Priorités :

``` text
LOW
MEDIUM
HIGH
URGENT
```

Fonctionnalités :

-   créer ;
-   consulter ;
-   modifier ;
-   supprimer ;
-   assigner ;
-   changer le statut ;
-   changer la priorité ;
-   ajouter des labels ;
-   définir une échéance ;
-   commenter.

------------------------------------------------------------------------

# 9. Détail d'une tâche

Route :

``` text
/app/projects/:projectId/tasks/:taskId
```

Exemple :

``` text
┌──────────────────────────────────────────┐
│ ← Back                                   │
│                                          │
│ Implement Frus navigation                │
│                                          │
│ Status       In Progress                 │
│ Priority     High                        │
│ Assignee     Kokou                       │
│ Due date     Sep 25                      │
│                                          │
│ Description                              │
│ Implement nested navigation...            │
│                                          │
│ ──────────────────────────────────────── │
│                                          │
│ Comments                                 │
│                                          │
│ Sarah                                     │
│ We should support nested routes.         │
│                                          │
│ [ Write a comment...              ]      │
└──────────────────────────────────────────┘
```

------------------------------------------------------------------------

# 10. Commentaires

Un utilisateur peut :

-   ajouter un commentaire ;
-   modifier son commentaire ;
-   supprimer son commentaire.

API :

``` http
GET    /api/v1/tasks/:taskId/comments
POST   /api/v1/tasks/:taskId/comments
PATCH  /api/v1/comments/:id
DELETE  /api/v1/comments/:id
```

------------------------------------------------------------------------

# 11. Membres d'un projet

Un projet possède des membres.

``` text
Project
├── Owner
├── Admin
└── Member
```

Owner/Admin peut :

-   inviter un utilisateur ;
-   retirer un membre ;
-   modifier son rôle.

Member peut :

-   consulter le projet ;
-   créer des tâches ;
-   modifier les tâches selon ses permissions.

------------------------------------------------------------------------

# 12. Invitations

Flow :

``` text
Owner
 │
 │ invite user@example.com
 ▼
Invitation
 │
 ▼
User receives notification
 │
 ├── Accept
 └── Decline
```

API :

``` http
POST /api/v1/projects/:id/invitations
GET  /api/v1/invitations
POST /api/v1/invitations/:id/accept
POST /api/v1/invitations/:id/decline
```

------------------------------------------------------------------------

# 13. Notifications

Centre de notifications :

``` text
🔔 Notifications

● Sarah assigned you "Implement router"
  2 minutes ago

● David commented on your task
  15 minutes ago

○ You were added to Frus Framework
  Yesterday
```

Types :

``` text
TASK_ASSIGNED
TASK_UPDATED
COMMENT_ADDED
PROJECT_INVITATION
MENTION
```

Fonctionnalités :

-   afficher les notifications ;
-   marquer une notification comme lue ;
-   tout marquer comme lu ;
-   ouvrir l'élément associé.

------------------------------------------------------------------------

# 14. Temps réel

Forge utilise :

``` text
HTTP
+
WebSocket
```

Exemple :

``` text
User A
   │
   │ moves Task #42
   ▼
Axum
   │
   │ broadcast
   ├──────────────► User B
   ├──────────────► User C
   └──────────────► User D
```

Les autres utilisateurs peuvent voir immédiatement :

-   changement de statut ;
-   nouveau commentaire ;
-   nouvelle tâche ;
-   modification ;
-   suppression ;
-   nouvelles notifications.

Le WebSocket est un élément important de la démonstration **Frus +
Axum**.

------------------------------------------------------------------------

# 15. Activity Feed

Chaque projet possède un historique :

``` text
Activity

Kokou created "Authentication"
Sarah moved "Router" to Review
David commented on "WebSocket"
Kokou assigned "Database" to Sarah
```

Événements possibles :

``` text
PROJECT_CREATED
MEMBER_ADDED
MEMBER_REMOVED
TASK_CREATED
TASK_UPDATED
TASK_DELETED
TASK_ASSIGNED
COMMENT_CREATED
COMMENT_UPDATED
COMMENT_DELETED
```

------------------------------------------------------------------------

# 16. Recherche

Recherche globale :

``` text
Search...
```

La recherche peut porter sur :

-   projets ;
-   tâches ;
-   membres.

Exemple :

``` text
"navigation"

Projects
  Frus Framework

Tasks
  Implement navigation
  Add nested navigation
```

------------------------------------------------------------------------

# 17. Profil

Route :

``` text
/app/profile
```

Informations :

``` text
Avatar
Full name
Username
Email
Bio
```

Fonctionnalités :

-   consulter son profil ;
-   modifier son nom ;
-   modifier sa photo ;
-   modifier sa bio ;
-   modifier certaines informations du compte.

------------------------------------------------------------------------

# 18. Settings

Route :

``` text
/app/settings
```

## Appearance

``` text
Theme
○ Light
● Dark
○ System
```

## Language

``` text
English
Français
```

## Notifications

``` text
Task assignments       ON
Comments               ON
Project invitations    ON
Mentions               ON
```

Cette section permet de démontrer le système de **thème et de
localisation de Frus**.

------------------------------------------------------------------------

# 19. Gestion des états

Chaque écran doit gérer explicitement :

``` text
Initial
Loading
Success
Empty
Error
```

Exemple :

``` text
Loading
   ↓
Success ──► Empty
   │
   └──────► Error
```

Le projet doit éviter les exemples qui montrent uniquement le "happy
path".

------------------------------------------------------------------------

# 20. Gestion des erreurs

Le backend retourne des erreurs structurées.

Exemple :

``` json
{
  "error": {
    "code": "PROJECT_NOT_FOUND",
    "message": "Project not found"
  }
}
```

Le frontend transforme l'erreur en interface utilisateur appropriée.

Exemple :

``` text
Unable to load project.

[ Try again ]
```

Les erreurs doivent être gérées au minimum pour :

-   validation ;
-   authentification ;
-   autorisation ;
-   ressource inexistante ;
-   réseau ;
-   serveur ;
-   timeout ;
-   session expirée.

------------------------------------------------------------------------

# 21. Architecture fonctionnelle du backend

Le backend Axum est organisé par domaines métier :

``` text
backend/
├── auth/
├── users/
├── projects/
├── members/
├── invitations/
├── tasks/
├── comments/
├── notifications/
├── activities/
└── realtime/
```

Chaque domaine peut suivre une architecture :

``` text
domain
application
infrastructure
presentation
```

Exemple :

``` text
tasks/
├── domain/
│   ├── entity.rs
│   └── repository.rs
│
├── application/
│   ├── create_task.rs
│   ├── update_task.rs
│   └── delete_task.rs
│
├── infrastructure/
│   └── postgres_repository.rs
│
└── presentation/
    ├── handlers.rs
    └── routes.rs
```

L'objectif est également de montrer comment organiser proprement un
backend Rust.

------------------------------------------------------------------------

# 22. API globale

Préfixe :

``` text
/api/v1
```

Principaux domaines :

``` text
/auth
/users
/projects
/projects/:id/members
/projects/:id/invitations
/projects/:id/tasks
/tasks/:id
/tasks/:id/comments
/notifications
/activities
/ws
```

------------------------------------------------------------------------

# 23. Modèle de données

Schéma simplifié :

``` text
User
 │
 ├────────────── ProjectMember ────────────── Project
 │                                             │
 │                                             ├── Task
 │                                             │    │
 │                                             │    └── Comment
 │                                             │
 │                                             └── Activity
 │
 └── Notification
```

Tables principales :

``` text
users
projects
project_members
invitations
tasks
comments
notifications
activities
```

------------------------------------------------------------------------

# 24. Routes Frus

``` text
/
├── /login
├── /register
│
└── /app
    ├── /dashboard
    ├── /projects
    │   ├── /:projectId
    │   ├── /:projectId/board
    │   ├── /:projectId/tasks
    │   ├── /:projectId/tasks/:taskId
    │   ├── /:projectId/members
    │   └── /:projectId/activity
    │
    ├── /notifications
    ├── /profile
    └── /settings
```

------------------------------------------------------------------------

# 25. Matrice des capacités Frus

  Capacité Frus       Utilisation dans Forge
  ------------------- --------------------------
  Widgets             Toute l'application
  Layout              Dashboard / Board
  Routing             Navigation
  State               Auth / Projects / Tasks
  HTTP                API Axum
  Forms               Login / Task / Project
  Validation          Forms
  Storage             Session / cache
  Theme               Light / Dark
  Localization        EN / FR
  Animation           Board / dialogs
  Async               API requests
  WebSocket           Collaboration temps réel
  Error handling      Network/API errors
  Testing             Unit / integration
  Responsive layout   Desktop / mobile

------------------------------------------------------------------------

# 26. MVP

Le projet doit être développé progressivement.

## Phase 1 --- Foundation

``` text
✓ Register
✓ Login
✓ Logout
✓ Dashboard
✓ Projects
✓ Project details
✓ Tasks
✓ Task details
✓ CRUD
✓ Navigation
✓ API Axum
✓ PostgreSQL
```

Objectif :

> établir la communication complète Frus → Axum → PostgreSQL.

------------------------------------------------------------------------

## Phase 2 --- Collaboration

``` text
✓ Project members
✓ Invitations
✓ Comments
✓ Notifications
✓ Search
✓ Filters
✓ Activity feed
```

Objectif :

> introduire les fonctionnalités collaboratives.

------------------------------------------------------------------------

## Phase 3 --- Real-time

``` text
✓ WebSocket
✓ Real-time task updates
✓ Real-time comments
✓ Real-time notifications
✓ Activity updates
```

Objectif :

> démontrer la communication temps réel entre Frus et Axum.

------------------------------------------------------------------------

## Phase 4 --- Offline

``` text
✓ Local storage
✓ Offline mode
✓ Synchronisation
✓ Optimistic updates
✓ Retry failed requests
✓ Conflict handling
```

Objectif :

> démontrer une application Frus capable de fonctionner avec une
> connectivité limitée.

------------------------------------------------------------------------

## Phase 5 --- Production quality

``` text
✓ Tests
✓ Accessibility
✓ Localization
✓ Responsive polish
✓ Performance
✓ Error recovery
✓ Documentation
✓ CI/CD
```

------------------------------------------------------------------------

# 27. Structure du repository

``` text
forge/
│
├── frontend/
│   ├── src/
│   │   ├── app/
│   │   ├── core/
│   │   └── features/
│   └── Cargo.toml
│
├── backend/
│   ├── src/
│   │   ├── auth/
│   │   ├── projects/
│   │   ├── tasks/
│   │   ├── comments/
│   │   └── realtime/
│   └── Cargo.toml
│
├── migrations/
│
├── docs/
│   ├── architecture.md
│   ├── frontend.md
│   ├── backend.md
│   ├── api.md
│   └── realtime.md
│
├── .env.example
├── Cargo.toml
└── README.md
```

------------------------------------------------------------------------

# 28. Documentation pédagogique

Le repository doit expliquer progressivement comment construire
l'application.

Parcours recommandé :

``` text
01. Create a Frus application
02. Create your first widget
03. Build layouts
04. Add routing
05. Manage application state
06. Create forms
07. Call an Axum API
08. Handle authentication
09. Build CRUD
10. Add project members
11. Add comments
12. Add WebSockets
13. Handle offline state
14. Test your application
15. Build and deploy
```

Chaque étape peut être accompagnée d'un commit ou d'un tag permettant de
retrouver l'état du projet à ce moment-là.

------------------------------------------------------------------------

# 29. Principes de conception

Forge doit respecter plusieurs principes :

### Simplicité

Le code doit être compréhensible par un développeur qui découvre Frus.

### Réalisme

Le projet doit utiliser de vrais flux :

``` text
UI → State → API → Database
```

et non des données entièrement mockées.

### Progressivité

Chaque fonctionnalité doit pouvoir être introduite indépendamment.

### Démonstration de Frus

Le projet doit privilégier les fonctionnalités natives de Frus
lorsqu'elles existent.

### Architecture propre

Le frontend et le backend doivent avoir une séparation claire des
responsabilités.

### Testabilité

Les domaines métier importants doivent pouvoir être testés
indépendamment de l'interface.

------------------------------------------------------------------------

# 30. Objectif final

Forge ne doit pas être simplement :

> "une application faite avec Frus".

Il doit devenir :

> **"l'application qui montre comment construire une vraie application
> avec Frus."**

Il sert simultanément de :

-   projet de démonstration ;
-   exemple officiel ;
-   documentation pratique ;
-   test d'intégration grandeur nature ;
-   vitrine du framework ;
-   terrain de test des nouvelles fonctionnalités Frus ;
-   point de départ pour les développeurs qui veulent apprendre Frus.

L'objectif final est qu'un développeur puisse cloner le repository,
lancer :

``` bash
frus run
```

et découvrir une application complète construite avec Frus, tout en
pouvant suivre son code pour apprendre progressivement le framework.
