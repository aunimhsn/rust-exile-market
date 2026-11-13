# Une copie refusée par le système, sous Windows : les tests

« Copy whisper » et « Copy the log » n'affichent « Copied » que si Windows a pris le texte. Ici, une petite commande garde le presse-papiers pour que la copie échoue : la pastille doit le dire, et un clic doit réessayer.

## Avant

1. Télécharge l'archive Windows `exile-market-windows-x64.zip` (lien donné par le propriétaire) et décompresse-la.
2. Lance `exile-market.exe`, puis ouvre le Bloc-notes pour coller.
3. **Avant de garder le presse-papiers**, sinon l'objet ne sera plus lu : copie dans le jeu un objet qui a des offres, clique sur la **loupe** si elles ne sont pas déjà là, et vérifie qu'un vendeur en ligne apparaît.
4. Ouvre PowerShell, et garde la même fenêtre pour tout le guide. Colle ces deux lignes, puis appuie sur Entrée : le presse-papiers reste gardé jusqu'à ce que tu répondes par Entrée à la question de PowerShell.

   ```powershell
   Add-Type -Name Clip -Namespace Test -MemberDefinition '[DllImport("user32.dll")] public static extern bool OpenClipboard(IntPtr h); [DllImport("user32.dll")] public static extern bool CloseClipboard();'
   [Test.Clip]::OpenClipboard([IntPtr]::Zero); Read-Host "Presse-papiers garde. Entree pour le liberer"; [Test.Clip]::CloseClipboard()
   ```

   Si la commande affiche `False`, une autre application tient déjà le presse-papiers : appuie sur Entrée, ferme cette application, puis relance la seconde ligne.

5. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log`.

## Les tests

1. **Le journal refusé** : presse-papiers gardé, ouvre le rapport de bug (le bug, à gauche de l'engrenage), puis clique sur **Copy the log** (« Copier le journal »).
   ✅ La pastille devient, en rouge, « Not copied, try again » (« Non copié, réessayer »), jamais « Copied » ; le log dit « The clipboard couldn't be written », sans le texte du journal. Environ une seconde après le début de la garde, la bande « The clipboard can't be read » s'affiche : c'est normal.

2. **Le journal, réessayé** : appuie sur Entrée dans PowerShell, reclique sur la pastille, puis colle dans le Bloc-notes.
   ✅ La bande disparaît, la pastille devient « ✓ Copied » (« ✓ Copié ») et le Bloc-notes reçoit tout le journal.

3. **Le whisper refusé** : ferme l'aperçu (Échap) et relance la seconde ligne de PowerShell. **Clique sur la ligne** d'une offre d'un vendeur en ligne pour épingler son infobulle, puis clique sur **Copy whisper** (« Copier le whisper »).
   ✅ La pastille devient, en rouge, « Not copied, try again » (« Non copié, réessayer »), et le reste tant que l'infobulle est ouverte, sans repasser seule à « Copy whisper ».

4. **Le whisper, réessayé** : appuie sur Entrée dans PowerShell (l'infobulle épinglée reste ouverte), reclique sur la pastille, puis colle dans le Bloc-notes.
   ✅ La pastille devient « Copied » (« Copié ») avec la coche pendant 2 secondes, et le Bloc-notes reçoit le message au vendeur (`@vendeur …`).

5. **Une autre offre** : ferme l'infobulle (Échap) et relance la seconde ligne de PowerShell. **Sans cliquer sur la ligne**, survole seulement une offre, clique sur la pastille de son infobulle, passe sur une autre offre, puis reviens. Appuie enfin sur Entrée dans PowerShell.
   ✅ Après l'échec, l'autre offre et l'offre d'origine proposent de nouveau « Copy whisper » (« Copier le whisper »).

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
