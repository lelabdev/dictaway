# dictaway

Dictation vocale pour Wayland. Appuie sur une touche, parle, appuie à nouveau — le texte apparaît dans ton application active.

Capture l'audio via `ffmpeg`, transcrit avec [whisper.cpp](https://github.com/ggerganov/whisper.cpp) (accélération GPU CUDA, pas de fichiers temporaires), tape le texte avec `wtype`. Affiche une visualisation en temps réel de la forme d'onde. Met en pause les médias pendant la dictée.

## Fonctionnalités

- Basé sur un raccourci : un touche pour démarrer/arrêter
- Visualisation en temps réel de la forme d'onde (GTK4 Layer Shell, dégradé ambre→turquoise)
- Détection automatique de la langue (Français, Anglais, Allemand, etc.)
- Téléchargement automatique du modèle whisper au premier lancement
- Accélération GPU CUDA via `whisper-rs`
- Pause/reprise automatique des médias via `playerctl`
- Pipeline audio en mémoire (pas de fichiers WAV temporaires)
- Filtrage des artefacts whisper (musique, ellipsis, etc.)
- Configuration flexible : modèle, périphérique audio et langue
- Filtres personnalisables pour ignorer des mots/expressions

## Prérequis

- Chaîne d'outils Rust + Cargo
- `ffmpeg` (capture audio)
- `wtype` (simulateur de clavier Wayland)
- `playerctl` (contrôle des médias)
- GPU NVIDIA + CUDA (optionnel, pour l'accélération GPU)

## Installation

### Depuis un binaire précompilé (Linux x86_64, CUDA activé)

```bash
curl -L https://github.com/lelabdev/dictaway/releases/latest/download/dictaway -o ~/.local/bin/dictaway && chmod +x ~/.local/bin/dictaway
```

### Compiler depuis les sources

```bash
cargo install --git https://github.com/lelabdev/dictaway --features cuda
```

### Manuellement depuis les sources

<details>
<summary>Compiler manuellement</summary>

```bash
git clone https://github.com/lelabdev/dictaway.git
cd dictaway
cargo build --release --features cuda
cp target/release/dictaway ~/.local/bin/
```

</details>

## Utilisation

### Mode CLI

```bash
dictaway                                        # basculer on/off
dictaway --lang en                             # forcer l'anglais cette fois
dictaway --model ~/chemin/vers/ggml-medium.bin   # utiliser un modèle spécifique
dictaway --device alsa_input.pci-001            # utiliser un périphérique audio spécifique
dictaway --stop                                 # forcer l'arrêt
```

**Premier appel** : commence l'écoute, met en pause les médias, transcrit et tape le texte en blocs de 3 secondes.

**Second appel** (ou Ctrl+C) : arrête, vide le texte restant, reprend les médias.

## Configuration

### Fichier de configuration principal

Crée `~/.config/dictaway/config` :

```bash
# Langue par défaut (fr, en, de, es, it, pt, nl, hi, auto)
# auto = détection automatique
lang=fr
```

**Langues disponibles** : `fr`, `en`, `de`, `es`, `it`, `pt`, `nl`, `hi`, `auto` (détection automatique).

**Remplacer pour un appel** : utilise `dictaway --lang en` ou `dictaway --lang auto`.

### Filtres de texte

dictaway filtre automatiquement les artefacts de transcription (bruit, musique, etc.). Tu peux ajouter tes propres filtres pour ignorer des mots/expressions spécifiques.

**Fichier de filtres personnalisés** :

Crée `~/.config/dictaway/filters` :

```bash
# Filtres personnalisés pour dictaway
# ~/.config/dictaway/filters
#
# Chaque ligne est interprétée comme une expression regex.
# Les correspondances sont remplacées par une chaîne vide.
#
# Exemples :
#
# Ignorer un mot spécifique
# \bmon_mot_inutile\b
#
# Ignorer plusieurs mots (alternatives)
# \b(mot_a|mot_b|mot_c)\b
#
# Remplacer une expression par vide
# (expression_inutile)

# Exemple : ignorer un mot spécifique
mon_mot_personnel

# Exemple : ignorer des expressions fréquentes
\b(et|est|sont)\b
```

**Comment ça marche** :

1. **Filtres internes** (toujours actifs) :
   - Mots : "Musique", "Music", "Bruit", "Noise", "Applaudissements", "Applause", "Rires", "Laughter", "BLANK_AUDIO", "blank_audio"
   - Expressions regex : `[...]`, `*...*`, `..`, `...`

2. **Filtres personnalisés** (fichier externe) :
   - Chaque ligne du fichier `~/.config/dictaway/filters` est une regex
   - Les correspondances sont supprimées
   - Utile pour ignorer des mots/expressions que tu ne veux pas voir

3. **Ordre d'application** :
   1. Regex internes (brackets, asterisks, ellipsis)
   2. Filtre des mots internes
   3. Filtres personnalisés
   4. Nettoyage final des espaces doubles

## Première exécution

Au premier lancement, si aucun modèle n'est trouvé, tu verras un sélecteur interactif :

```
🎤 Aucun modèle whisper trouvé. Choisissons-en un !

  #  Modèle       Taille     GPU VRAM   Vitesse    Qualité
  ──────────────────────────────────────────────────────
  1  tiny        75 MB     < 1 GB     ⚡⚡⚡   Basique
  2  base        142 MB    ~1 GB      ⚡⚡     Correct
  3  small       466 MB    ~2 GB      ⚡        Bon ← recommandé
  4  medium      1.5 GB    ~5 GB      Lent      Très bon
  5  large-v3-turbo  1.5 GB   ~8 GB      Rapide   Excellent
  6  large-v3    2.9 GB    ~10 GB     Très lent   Excellent

  💡 Pas de GPU ? Tous les modèles fonctionnent aussi en CPU (juste plus lent).
```

Choisis un numéro (par défaut : 3 = small).

Le modèle est téléchargé automatiquement et réutilisé lors des prochaines exécutions.

## Configuration du modèle

Dans `~/.config/dictaway/config`, tu peux spécifier le modèle :

```bash
model=small
lang=fr
```

**Modèles disponibles** :

| Nom      | Fichier          | Taille   | GPU VRAM | Vitesse   | Qualité |
|----------|------------------|----------|-----------|----------|---------|
| tiny     | ggml-tiny.bin     | 75 MB    | < 1 GB    | ⚡⚡⚡  | Basique  |
| base     | ggml-base.bin     | 142 MB   | ~1 GB     | ⚡⚡     | Correct  |
| small    | ggml-small.bin     | 466 MB   | ~2 GB     | ⚡        | Bon     | ← recommandé |
| medium   | ggml-medium.bin    | 1.5 GB   | ~5 GB     | Lent      | Très bon|
| large-v3-turbo | ggml-large-v3-turbo.bin | 1.5 GB   | ~8 GB     | Rapide   | Excellent|
| large-v3 | ggml-large-v3.bin      | 2.9 GB   | ~10 GB    | Très lent | Excellent|

## Raccourcis clavier

### Exemple pour MangoWM

```bash
bind=SUPER,d,spawn,dictaway
```

## Architecture

```
ffmpeg (PulseAudio, 16kHz mono)
  → ring buffer (échantillons f32)
    → compteur de volume → overlay (GTK4 Layer Shell, temps réel)
      → whisper-rs + CUDA (transcription, blocs de 3s)
            → filtre de texte (supprime les artefacts)
              → wtype (tape le texte)

playerctl --all-players pause/play (contrôle des médias)
```

### Overlay

Une visualisation flottante de la forme d'onde apparaît en bas de l'écran pendant la dictée :

- 9 barres animées avec dégradé de couleur ambre→turquoise basé sur le niveau de voix
- Réponse en temps réel (fenêtre audio de 62ms, rendu à 40fps)
- Point d'enregistrement clignotant (REC)
- Masquage automatique quand la dictée s'arrête

## Service systemd (auto-démarrage)

Tu peux configurer dictaway pour qu'il se lance automatiquement à chaque session.

### Installation du service

```bash
# Copier le fichier de service
mkdir -p ~/.config/systemd/user/
cp dictaway.service ~/.config/systemd/user/

# Recharger systemd
systemctl --user daemon-reload

# Activer le démarrage automatique
systemctl --user enable dictaway

# Démarrer maintenant
systemctl --user start dictaway
```

### Utilisation du service

```bash
# Démarrer
systemctl --user start dictaway

# Arrêter
systemctl --user stop dictaway

# Redémarrer
systemctl --user restart dictaway

# Voir l'état
systemctl --user status dictaway

# Voir les logs
journalctl -u dictaway
```

### Avantages du service

- **Démarrage automatique** : dictaway se lance à chaque connexion
- **Redémarrage automatique** : si ça plante, il redémarre automatiquement
- **Logs centralisés** : `journalctl -u dictaway` donne un historique complet
- **Gestion simplifiée** : `systemctl --user restart dictaway` au lieu de kill/start
- **État visible** : `systemctl --user status dictaway` permet de savoir si ça tourne

### Fichier de service

Un fichier `dictaway.service` est inclus dans le projet pour faciliter l'installation.

## Dépannage

### Voir les logs

```bash
journalctl -u dictaway -f
```

### Problèmes de transcription

Si la transcription contient des artefacts :
- **Musique/Bruit** : Whisper peut mal distinguer la voix du fond sonore
- **Ellipsis** : Whisper remplace parfois les pauses par des points de suspension (...)
- **Mots manquants** : Si tu parles vite, Whisper peut sauter des mots

Solutions :
- Utiliser un meilleur modèle (medium ou large-v3-turbo)
- Ajuster la distance du micro
- Parler plus lentement et clairement
- Ajouter des filtres personnalisés pour les artefacts récurrents

## Mises à jour

### Entre versions

dictaway essaie de préserver la configuration entre les mises à jour :

- **Installation** : `cargo install --path .` remplace le binaire sans toucher à la configuration
- **Service** : Après installation, recharger le service avec `systemctl --user daemon-reload`
- Pas besoin de re-créer la configuration

## Licence

MIT
