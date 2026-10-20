# Nova Control — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1623519 |
| Taille du fichier | 1.9 Mo |
| Intentions | 30 |
| Exemples d'entraînement | 27087 |
| Durée d'entraînement | 81 s |
| Temps d'analyse moyen | 0.08 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 99.3 % | 95.5 % |
| Test (vocabulaire jamais vu) | 98.9 % | 74.0 % |
| Phrases réalistes écrites à la main | 100.0 % | 70.0 % |

## Intentions

`autre`, `annuler`, `capture_ecran`, `cliquer`, `coller`, `copier`, `couper`, `creer_dossier`, `defiler`, `ecrire`, `enregistrer`, `envoyer_mail`, `envoyer_message`, `fermer_app`, `fermer_onglet`, `lecture_pause`, `menu`, `muet`, `nouveau_document`, `nouvel_onglet`, `nouvelle_note`, `ouvrir_app`, `ouvrir_site`, `piste_suivante`, `plein_ecran`, `recherche_web`, `rechercher_page`, `retablir`, `tout_selectionner`, `volume`

## Erreurs restantes (phrases réalistes)

- « envoie un mail à paul@exemple.fr pour dire que je serai en retard » : infos [("destinataire", "paul@exemple.fr"), ("message", "je serai en retard")] au lieu de [("destinataire", "paul@exemple.fr"), ("message", "que je serai en retard")]
- « crée une note Courses avec pain, lait, œufs » : infos [("contenu", "pain, lait"), ("contenu", "œufs"), ("texte", ","), ("titre", "courses")] au lieu de [("contenu", "pain, lait, œufs"), ("titre", "courses")]
- « écris bonjour tout le monde » : infos [("direction", "monde"), ("texte", "bonjour tout")] au lieu de [("texte", "bonjour tout le monde")]
- « envoie un message à Tom sur Discord : on joue ce soir ? » : infos [("app", "discord"), ("destinataire", "tom"), ("message", "on joue ce soir")] au lieu de [("app", "discord"), ("destinataire", "tom"), ("message", "on joue ce soir ?")]
- « écris à maman sur WhatsApp pour lui dire que j'arrive » : infos [("app", "whatsapp"), ("destinataire", "maman")] au lieu de [("app", "whatsapp"), ("destinataire", "maman"), ("message", "j'arrive")]
- « dis à Léa sur insta que la soirée est annulée » : infos [("app", "insta"), ("destinataire", "léa"), ("message", "la")] au lieu de [("app", "insta"), ("destinataire", "léa"), ("message", "la soirée est annulée")]
