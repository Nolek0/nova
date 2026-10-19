# Nova 1 — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1620422 |
| Taille du fichier | 1.9 Mo |
| Intentions | 23 |
| Exemples d'entraînement | 28100 |
| Durée d'entraînement | 68 s |
| Temps d'analyse moyen | 0.07 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 100.0 % | 97.2 % |
| Test (vocabulaire jamais vu) | 88.8 % | 72.7 % |
| Phrases réalistes écrites à la main | 98.6 % | 92.0 % |

## Intentions

`autre`, `annuler_minuteur`, `au_revoir`, `blague`, `calcul`, `capacites`, `chercher_fichier`, `comment_ca_va`, `conversion`, `date`, `heure`, `identite`, `infos_systeme`, `lancer_de`, `meteo`, `minuteur`, `note`, `ouvrir_app`, `ouvrir_site`, `pile_ou_face`, `recherche_web`, `remerciement`, `salutation`

## Erreurs restantes (phrases réalistes)

- « bon je vais dormir, bonne nuit » → autre (54 %) au lieu de au_revoir
- « crée une note réviser l'anglais ce soir » : infos [("contenu", "réviser l'anglais"), ("contenu", "soir")] au lieu de [("contenu", "réviser l'anglais ce soir")]
- « cherche sur internet la coupe du monde 2026 » : infos [("requete", "2026"), ("requete", "la")] au lieu de [("requete", "la coupe du monde 2026")]
