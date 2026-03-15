# ADR-021 : Système de Prefabs

- **Statut :** Acceptée
- **Date :** 2026-03-15
- **Concerne :** v0.3

## Contexte

Un prefab (ou template) est une entité sauvegardée comme modèle réutilisable. Placer 50 ennemis identiques ne devrait pas nécessiter de configurer 50 fois les mêmes propriétés. Les prefabs permettent de définir un "Enemy" une fois et de l'instancier partout.

## Décision

**Prefabs comme entités templates sérialisées en JSON, avec instanciation et héritage.**

### Modèle

```
Prefab {
    name: String,
    entity_data: EntityData,        // propriétés par défaut
    behaviors: Vec<BehaviorConfig>, // behaviors attachés
    event_sheet: Option<String>,    // event sheet référencé
}

PrefabInstance {
    prefab_name: String,
    overrides: HashMap<String, Value>,  // propriétés surchargées
}
```

### Workflow

1. **Créer** : configurer une entité dans l'éditeur (sprite, taille, behaviors, etc.), clic droit → "Save as Prefab"
2. **Instancier** : drag depuis le panneau Prefabs vers le viewport. L'instance hérite de toutes les propriétés du prefab.
3. **Surcharger** : modifier une propriété sur l'instance. Elle devient une "override" — les propriétés non surchargées suivent le prefab.
4. **Propager** : modifier le prefab → toutes les instances non-surchargées se mettent à jour.

### Stockage
Les prefabs sont stockés dans un dossier `prefabs/` du projet, un fichier JSON par prefab. Les instances dans la scène référencent le prefab par nom.

## Conséquences

### Positives
- Réutilisation massive — un ennemi défini une fois, instancié partout
- Les changements au prefab propagent à toutes les instances
- Les overrides permettent des variantes (ennemi rouge = même prefab, couleur surchargée)
- Compatible MCP (`create_prefab`, `instantiate_prefab`)

### Négatives
- Complexité de l'héritage et de la propagation
- Les overrides doivent être trackées proprement (diff entre instance et prefab)
- Les prefabs imbriqués (prefab contenant des instances d'autres prefabs) sont hors scope v0.3
