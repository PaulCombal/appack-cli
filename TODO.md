TODO:

AVANT Que j'oublie ce que j'étais en train de faire:
- Test de packager proprement (mettre tous scripts dans C) puis recréer l'appack
- Disable logging
- Refaire de A à Z ? -> remove optionalfeatures etc -> se renseigner sur le fait de minimiser l'espace d'un qcow2
- Enable virgl in test/configure assets

Release:
- Mettre le process d'extraction dans /tmp pendant l'install et juste rename
- Write lots of documentation
- creator --> rename myapp to ms-cmd
- Renommer Appack.yaml en AppPackBuild.yaml
- Faire un script .ps1 qui va copier les .vbs dans C + appliquer les .reg

Nice to have:
- appack info <zip> --> extract readme in /tmp or so
