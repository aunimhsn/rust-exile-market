# L'exécutable, sous Linux : les tests (17a)

L'exécutable s'appelle désormais `exile-market` (sans le préfixe `rust-` du dépôt), et ce nom est aussi l'identifiant de sa fenêtre. Sa fenêtre porte l'icône de l'application, l'emblème en bronze. Quand la fenêtre ne peut pas s'ouvrir, une boîte de dialogue du système le dit (kdialog sous KDE, sinon zenity), avec le même texte dans le terminal et le journal. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche.

## Avant

1. Ferme Exile Market s'il tourne.
2. Dans le dossier du projet, sur la branche de l'étape : `cargo build --release`.
3. Le log est dans `~/.local/state/exile-market/exile-market.log`.

## Les tests

1. **Le nom** : lance `target/release/exile-market`.
   ✅ La fenêtre s'ouvre comme avant, avec tes réglages (langue, ligue, touche du raccourci) : le dossier des réglages n'a pas changé.

2. **L'icône** : regarde la barre des tâches, puis fais Alt+Tab.
   ✅ L'emblème en bronze, sans tuile, à la place de l'icône générique ; le même dans le sélecteur de fenêtres. L'emblème de la barre de titre de l'application reste blanc.

3. **La fenêtre ne s'ouvre pas** : ferme Exile Market, puis lance-le sans écran X11 : `env -u DISPLAY target/release/exile-market`.
   ✅ Une boîte de dialogue de KDE, titrée « Exile Market », avec le signe d'erreur du système : « Exile Market ne peut pas ouvrir sa fenêtre : il lui faut X11, ou XWayland sur un bureau Wayland. », puis « Installez XWayland… », puis « Détails : winit EventLoopError: os error: Failed to open connection to X server » (sans chemin de fichier). Le même texte dans le terminal. Après OK, le programme se termine ; le log écrit `The window failed (Display): …`.

4. **Avec zenity** (s'il est installé : `which zenity`) : `env -u DISPLAY XDG_CURRENT_DESKTOP=GNOME target/release/exile-market`.
   ✅ La même boîte, dessinée par zenity cette fois (l'aspect de GNOME).

5. **Sans boîte de dialogue** : `env -u DISPLAY PATH=/nonexistent target/release/exile-market`.
   ✅ Aucune boîte ; le message dans le terminal seulement, et le log écrit `No dialog of the system could show the message of the window`.

6. **Tes propres scénarios** : ce que tu veux essayer d'autre.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le nom | | |
| 2. L'icône | | |
| 3. La fenêtre ne s'ouvre pas | | |
| 4. Avec zenity | | |
| 5. Sans boîte de dialogue | | |
| 6. Tes propres scénarios | | |
