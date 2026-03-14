# ADR-004 : kira comme bibliothèque audio

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1
- **Dépend de :** ADR-001 (Rust)

## Contexte

Le moteur a besoin de charger et jouer des fichiers audio (WAV pour les SFX, OGG Vorbis pour la musique), avec contrôle de volume, lecture simultanée de 16+ sons, et les bases pour des features futures (audio spatial 2D, tweening, streaming).

## Options considérées

### miniaudio (via bindings Rust)
- **Pour :** single-header C, extrêmement portable, lightweight. Supporte WAV, OGG, MP3, FLAC. Mixeur intégré. Le choix standard en C/C++.
- **Contre :** C-natif (FFI depuis Rust). Pas de features game-spécifiques (tweening de paramètres, horloge de synchronisation). Il faudrait construire ces features par-dessus.

### rodio
- **Pour :** Rust pur. Simple d'utilisation. Basé sur cpal (cross-platform audio library).
- **Contre :** très bas niveau. Pas de mixeur game-friendly. Pas de tweening, pas de clocks, pas de streaming sans code custom. On finirait par reconstruire kira par-dessus rodio.

### kira
- **Pour :** Rust natif. Conçu spécifiquement pour les jeux. Tweening de paramètres intégré (volume, pitch, panning). Horloge de synchronisation pour la musique rythmique. Streaming intégré. Mixeur avec routing. Architecture basée sur des "arrangements" qui correspondent au modèle mental du game dev.
- **Contre :** dépendance sur cpal (partagée avec rodio). Moins de formats supportés nativement que miniaudio (WAV et OGG couvrent le MVP). Communauté plus petite que miniaudio.

### SDL3_mixer
- **Pour :** intégré avec SDL3 (déjà utilisé pour le windowing). API simple pour le cas d'usage basique.
- **Contre :** C-natif (FFI). API limitée pour les cas avancés. Pas de tweening. Couples l'audio au choix de windowing (mauvaise séparation des responsabilités).

## Décision

**kira.**

1. **Conçu pour les jeux.** kira offre nativement le tweening de volume/pitch/panning, les clocks de synchronisation, et le streaming. Ce sont des features qu'on aurait dû construire nous-mêmes avec rodio ou miniaudio. kira les fournit prêtes à l'emploi.

2. **Rust pur.** Pas de FFI. S'intègre naturellement dans l'architecture. Les types Rust (handles, paramètres) sont expressifs et safe.

3. **Pas de reconstruction.** Avec rodio, on finirait par construire un "kira maison" moins bon. Avec miniaudio, on passerait du temps en FFI wrappers pour arriver au même résultat. kira est le bon niveau d'abstraction.

## Conséquences

### Positives
- Features game audio prêtes à l'emploi (tweening, clocks, streaming)
- Pas de FFI, intégration Rust native
- Base solide pour les features audio avancées (v1.5 : audio spatial, bus DSP)

### Négatives
- Dépendance sur cpal pour le backend audio plateforme
- Moins de formats supportés nativement (WAV + OGG suffisent pour le MVP)
- Communauté plus petite que miniaudio (mais active et maintenue)
