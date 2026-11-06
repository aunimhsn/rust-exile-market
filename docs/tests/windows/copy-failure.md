# Une copie refusée par le système, sous Windows : les tests

« Copy whisper » et « Copy the log » n'affichent « Copied » que si Windows a pris le texte. Ici, une petite commande garde le presse-papiers pour que la copie échoue : la pastille doit le dire, et un clic doit réessayer.

## Avant

1. Télécharge l'archive Windows `exile-market-test-x86_64-pc-windows-msvc.zip` (lien donné par le propriétaire) et décompresse-la.
2. Lance `exile-market.exe`, puis ouvre le Bloc-notes pour coller.
3. Ouvre PowerShell et colle ces deux lignes : le presse-papiers reste gardé jusqu'à ce que PowerShell ait reçu Entrée à sa question.

   ```powershell
   Add-Type -Name Clip -Namespace Test -MemberDefinition '[DllImport("user32.dll")] public static extern bool OpenClipboard(IntPtr h); [DllImport("user32.dll")] public static extern bool CloseClipboard();'
   [Test.Clip]::OpenClipboard([IntPtr]::Zero); Read-Host "Presse-papiers garde. Entree pour le liberer"; [Test.Clip]::CloseClipboard()
   ```

   Si la commande affiche `False`, une autre application tient déjà le presse-papiers : ferme-la et relance la seconde ligne. Pendant ce temps, la bande « The clipboard can't be read » peut s'afficher dans Exile Market : c'est normal, et elle disparaît ensuite.

4. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log`.

## Les tests

1. **Le journal refusé** : presse-papiers gardé, ouvre le rapport de bug (le bug, à gauche de l'engrenage), puis clique sur **Copy the log** (« Copier le journal »).
   ✅ La pastille devient, en rouge, « Not copied, try again » (« Non copié, réessayer »), jamais « Copied » ; le log dit « The clipboard couldn't be written », sans le texte du journal.

2. **Le journal, réessayé** : appuie sur Entrée dans PowerShell, reclique sur la pastille, puis colle dans le Bloc-notes.
   ✅ La pastille devient « ✓ Copied » (« ✓ Copié ») et le Bloc-notes reçoit tout le journal.

3. **Le whisper refusé** : relance la seconde ligne de PowerShell. Survole une offre d'un vendeur en ligne, puis clique sur **Copy whisper** (« Copier le whisper »).
   ✅ La pastille devient, en rouge, « Not copied, try again » (« Non copié, réessayer »), et le reste tant que l'infobulle est ouverte, sans repasser seule à « Copy whisper ».

4. **Le whisper, réessayé** : appuie sur Entrée dans PowerShell, reclique sur la pastille, puis colle dans le Bloc-notes.
   ✅ La pastille devient « Copied » (« Copié ») avec la coche pendant 2 secondes, et le Bloc-notes reçoit le message au vendeur (`@vendeur …`).

5. **Une autre offre** : refais le test 3, puis passe sur une autre offre et reviens (presse-papiers encore gardé), puis libère-le.
   ✅ L'autre offre et l'offre d'origine proposent de nouveau « Copy whisper » (« Copier le whisper »).

6. **Tes propres scénarios** : essaie ce qui te passe par la tête (un autre journal choisi après un échec, l'échec dans la fenêtre par-dessus le jeu…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le journal refusé | | |
| 2. Le journal, réessayé | | |
| 3. Le whisper refusé | | |
| 4. Le whisper, réessayé | | |
| 5. Une autre offre | | |
| 6. Tes propres scénarios | | |
