//! Complete aligned catalogs. Unknown language codes fall back to English.
pub const LANGUAGES: [(&str, &str); 6] = [
    ("en", "English"),
    ("ru", "Русский"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("it", "Italiano"),
];

pub fn tr<'a>(language: &str, key: &'a str) -> &'a str {
    let column = LANGUAGES
        .iter()
        .position(|(code, _)| *code == language)
        .unwrap_or(0);
    MESSAGES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, values)| values[column])
        .unwrap_or(key)
}

pub const MESSAGES: &[(&str, [&str; 6])] = &[
    (
        "licenses",
        [
            "Licenses",
            "Лицензии",
            "Licences",
            "Lizenzen",
            "Licencias",
            "Licenze",
        ],
    ),
    (
        "export_licenses",
        [
            "Export license notices…",
            "Сохранить тексты лицензий…",
            "Exporter les licences…",
            "Lizenzhinweise exportieren…",
            "Exportar licencias…",
            "Esporta licenze…",
        ],
    ),
    (
        "zoom_in",
        [
            "Zoom in",
            "Увеличить",
            "Agrandir",
            "Vergrößern",
            "Acercar",
            "Ingrandisci",
        ],
    ),
    (
        "zoom_out",
        [
            "Zoom out",
            "Уменьшить",
            "Réduire",
            "Verkleinern",
            "Alejar",
            "Riduci",
        ],
    ),
    (
        "keyboard_canvas",
        [
            "Tab to the canvas; arrow keys pan, Home fits the sheet.",
            "Выберите схему клавишей Tab: стрелки сдвигают, Home вписывает лист.",
            "Tab vers le plan ; flèches pour déplacer, Origine pour ajuster.",
            "Mit Tab zur Zeichnung; Pfeile verschieben, Pos1 passt die Platte ein.",
            "Tab al plano; flechas para desplazar, Inicio para ajustar.",
            "Tab sul piano; frecce per spostare, Home per adattare.",
        ],
    ),
    (
        "installing",
        [
            "Preparing installation…",
            "Подготовка установки…",
            "Préparation de l’installation…",
            "Installation vorbereiten…",
            "Preparando la instalación…",
            "Preparazione dell’installazione…",
        ],
    ),
    (
        "csv_precision",
        [
            "Use fewer decimal places in CSV dimensions.",
            "Уменьшите число десятичных знаков в размерах CSV.",
            "Réduisez le nombre de décimales des dimensions CSV.",
            "Weniger Nachkommastellen in CSV-Maßen verwenden.",
            "Usa menos decimales en las medidas CSV.",
            "Usa meno decimali nelle dimensioni CSV.",
        ],
    ),
    (
        "csv_units",
        [
            "An optional unit column accepts mm, cm or in. Exports use exact millimetres with a unit column.",
            "Необязательный столбец unit: mm, cm или in. Экспорт сохраняет точные миллиметры и столбец единиц.",
            "La colonne facultative unit accepte mm, cm ou in. L’export conserve les millimètres exacts et l’unité.",
            "Optionale Spalte unit: mm, cm oder in. Export nutzt exakte Millimeter mit Einheitenspalte.",
            "La columna opcional unit acepta mm, cm o in. La exportación usa milímetros exactos y su unidad.",
            "La colonna facoltativa unit accetta mm, cm o in. L’esportazione usa millimetri esatti e relativa unità.",
        ],
    ),
    (
        "csv_unit",
        [
            "CSV units must be mm, cm or in.",
            "В CSV единицы: mm, cm или in.",
            "Les unités CSV doivent être mm, cm ou in.",
            "CSV-Einheit muss mm, cm oder in sein.",
            "Las unidades CSV deben ser mm, cm o in.",
            "Le unità CSV devono essere mm, cm o in.",
        ],
    ),
    (
        "tagline",
        [
            "Make every sheet count",
            "Каждый лист — с пользой",
            "Chaque panneau compte",
            "Jede Platte zählt",
            "Aprovecha cada tablero",
            "Ogni pannello conta",
        ],
    ),
    (
        "inputs",
        [
            "Stock & parts",
            "Листы и детали",
            "Panneaux et pièces",
            "Platten und Teile",
            "Tableros y piezas",
            "Pannelli e pezzi",
        ],
    ),
    (
        "constraints",
        [
            "Cut settings",
            "Параметры реза",
            "Réglages de coupe",
            "Schnitteinstellungen",
            "Ajustes de corte",
            "Impostazioni di taglio",
        ],
    ),
    (
        "plan",
        [
            "Cutting plan",
            "План раскроя",
            "Plan de découpe",
            "Zuschnittplan",
            "Plan de corte",
            "Piano di taglio",
        ],
    ),
    (
        "settings",
        [
            "Settings",
            "Настройки",
            "Paramètres",
            "Einstellungen",
            "Configuración",
            "Impostazioni",
        ],
    ),
    ("new", ["New", "Новый", "Nouveau", "Neu", "Nuevo", "Nuovo"]),
    (
        "open",
        ["Open", "Открыть", "Ouvrir", "Öffnen", "Abrir", "Apri"],
    ),
    (
        "save",
        [
            "Save",
            "Сохранить",
            "Enregistrer",
            "Speichern",
            "Guardar",
            "Salva",
        ],
    ),
    (
        "save_as",
        [
            "Save as…",
            "Сохранить как…",
            "Enregistrer sous…",
            "Speichern unter…",
            "Guardar como…",
            "Salva con nome…",
        ],
    ),
    (
        "project",
        [
            "Project",
            "Проект",
            "Projet",
            "Projekt",
            "Proyecto",
            "Progetto",
        ],
    ),
    (
        "project_name",
        [
            "Project name",
            "Название проекта",
            "Nom du projet",
            "Projektname",
            "Nombre del proyecto",
            "Nome del progetto",
        ],
    ),
    (
        "material",
        [
            "Material / thickness",
            "Материал / толщина",
            "Matériau / épaisseur",
            "Material / Stärke",
            "Material / espesor",
            "Materiale / spessore",
        ],
    ),
    (
        "unit",
        [
            "Display units",
            "Единицы размеров",
            "Unités d’affichage",
            "Anzeigeeinheiten",
            "Unidades",
            "Unità di misura",
        ],
    ),
    (
        "stock",
        [
            "Available stock",
            "Доступные листы",
            "Panneaux disponibles",
            "Verfügbare Platten",
            "Tableros disponibles",
            "Pannelli disponibili",
        ],
    ),
    (
        "parts",
        [
            "Parts to cut",
            "Детали для раскроя",
            "Pièces à découper",
            "Benötigte Teile",
            "Piezas a cortar",
            "Pezzi da tagliare",
        ],
    ),
    (
        "add_stock",
        [
            "Add sheet",
            "Добавить лист",
            "Ajouter un panneau",
            "Platte hinzufügen",
            "Añadir tablero",
            "Aggiungi pannello",
        ],
    ),
    (
        "add_part",
        [
            "Add part",
            "Добавить деталь",
            "Ajouter une pièce",
            "Teil hinzufügen",
            "Añadir pieza",
            "Aggiungi pezzo",
        ],
    ),
    (
        "name",
        ["Name", "Название", "Nom", "Name", "Nombre", "Nome"],
    ),
    (
        "width",
        ["Width", "Ширина", "Largeur", "Breite", "Ancho", "Larghezza"],
    ),
    (
        "height",
        ["Height", "Высота", "Hauteur", "Höhe", "Alto", "Altezza"],
    ),
    (
        "quantity",
        [
            "Quantity",
            "Количество",
            "Quantité",
            "Anzahl",
            "Cantidad",
            "Quantità",
        ],
    ),
    (
        "rotate",
        [
            "Allow rotation",
            "Разрешить поворот",
            "Rotation permise",
            "Drehung erlauben",
            "Permitir giro",
            "Consenti rotazione",
        ],
    ),
    (
        "rotated",
        [
            "Rotated 90°",
            "Поворот на 90°",
            "Tournée de 90°",
            "Um 90° gedreht",
            "Girada 90°",
            "Ruotato di 90°",
        ],
    ),
    (
        "grain",
        [
            "Grain direction",
            "Направление волокон",
            "Sens du fil",
            "Faserrichtung",
            "Dirección de veta",
            "Direzione della venatura",
        ],
    ),
    (
        "grain_none",
        [
            "Unrestricted",
            "Без ограничения",
            "Sans contrainte",
            "Unbeschränkt",
            "Sin restricción",
            "Senza vincoli",
        ],
    ),
    (
        "grain_width",
        [
            "Along width →",
            "По ширине →",
            "Selon la largeur →",
            "Entlang der Breite →",
            "A lo ancho →",
            "Lungo la larghezza →",
        ],
    ),
    (
        "grain_height",
        [
            "Along height ↓",
            "По высоте ↓",
            "Selon la hauteur ↓",
            "Entlang der Höhe ↓",
            "A lo alto ↓",
            "Lungo l’altezza ↓",
        ],
    ),
    (
        "remove",
        [
            "Remove",
            "Удалить",
            "Supprimer",
            "Entfernen",
            "Eliminar",
            "Rimuovi",
        ],
    ),
    (
        "example",
        [
            "Try a shelf project",
            "Пример: полки",
            "Exemple : étagères",
            "Beispiel: Regal",
            "Ejemplo de estantes",
            "Esempio: scaffale",
        ],
    ),
    (
        "empty_input",
        [
            "Start with your sheet sizes and the parts you need.",
            "Укажите размеры листов и нужных деталей.",
            "Indiquez les panneaux et les pièces nécessaires.",
            "Geben Sie Ihre Platten und benötigten Teile ein.",
            "Introduce los tableros y las piezas necesarias.",
            "Inserisci i pannelli e i pezzi necessari.",
        ],
    ),
    (
        "next",
        [
            "Continue",
            "Продолжить",
            "Continuer",
            "Weiter",
            "Continuar",
            "Continua",
        ],
    ),
    (
        "back",
        ["Back", "Назад", "Retour", "Zurück", "Atrás", "Indietro"],
    ),
    (
        "kerf",
        [
            "Saw kerf",
            "Ширина пропила",
            "Trait de scie",
            "Schnittfuge",
            "Ancho de corte",
            "Spessore del taglio",
        ],
    ),
    (
        "trim",
        [
            "Minimum edge trim",
            "Минимальная обрезка краёв",
            "Délignage minimal des bords",
            "Mindestbeschnitt je Kante",
            "Recorte mínimo por borde",
            "Rifilatura minima per bordo",
        ],
    ),
    (
        "min_offcut",
        [
            "Minimum offcut side",
            "Минимальная сторона остатка",
            "Côté minimal des chutes",
            "Mindestseite für Reststücke",
            "Lado mínimo del sobrante",
            "Lato minimo degli sfridi",
        ],
    ),
    (
        "kerf_help",
        [
            "Material removed by the blade. A new cut needs the full kerf.",
            "Материал, удаляемый пилой. На каждый рез нужен полный пропил.",
            "Matière retirée par la lame. Chaque coupe exige un trait complet.",
            "Materialverlust durch das Sägeblatt. Jeder Schnitt braucht die volle Fuge.",
            "Material eliminado por la sierra. Cada corte necesita todo su ancho.",
            "Materiale rimosso dalla lama. Ogni taglio richiede lo spessore completo.",
        ],
    ),
    (
        "trim_help",
        [
            "Includes kerf; use zero or at least one kerf. A trailing trim may grow by less than a kerf to avoid thin slivers; follow the cut sequence.",
            "Включает пропил; задайте ноль или не меньше пропила. Для устранения тонкой полоски конечная обрезка может вырасти менее чем на пропил; следуйте порядку резов.",
            "Inclut le trait ; zéro ou au moins un trait. Le bord final peut être rogné davantage de moins d’un trait pour éviter une fine chute ; suivez la séquence.",
            "Inklusive Fuge; null oder mindestens eine Fuge. Endbeschnitt kann um weniger als eine Fuge wachsen, um dünne Streifen zu vermeiden; Schnittfolge beachten.",
            "Incluye el corte; cero o al menos un ancho de sierra. El recorte final puede aumentar menos de un corte para evitar tiras finas; sigue la secuencia.",
            "Include il taglio; zero o almeno uno spessore di lama. Il bordo finale può aumentare meno di un taglio per evitare strisce sottili; segui la sequenza.",
        ],
    ),
    (
        "offcut_help",
        [
            "Both sides must meet this size to list a reusable offcut. This does not change placement.",
            "Обе стороны остатка должны быть не меньше этого размера. На размещение это не влияет.",
            "Les deux côtés doivent atteindre cette taille. Le placement reste inchangé.",
            "Beide Seiten müssen diese Größe erreichen. Die Platzierung bleibt gleich.",
            "Ambos lados deben alcanzar este tamaño. No modifica la colocación.",
            "Entrambi i lati devono raggiungere questa misura. Non cambia la disposizione.",
        ],
    ),
    (
        "grain_help",
        [
            "A directional part needs a sheet with a matching grain direction, including after rotation.",
            "Для детали с волокнами нужен лист с подходящим направлением, в том числе после поворота.",
            "Une pièce orientée exige un panneau au fil compatible, même après rotation.",
            "Teile mit Faserrichtung benötigen eine passende Platte, auch nach einer Drehung.",
            "Una pieza con veta requiere un tablero de dirección compatible, también al girarla.",
            "Un pezzo con venatura richiede un pannello compatibile, anche dopo la rotazione.",
        ],
    ),
    (
        "calculate",
        [
            "Create cutting plan",
            "Рассчитать раскрой",
            "Créer le plan",
            "Zuschnitt berechnen",
            "Crear plan de corte",
            "Crea piano di taglio",
        ],
    ),
    (
        "calculating",
        [
            "Comparing layouts…",
            "Сравниваю варианты…",
            "Comparaison des plans…",
            "Anordnungen vergleichen…",
            "Comparando planos…",
            "Confronto delle disposizioni…",
        ],
    ),
    (
        "cancel",
        [
            "Cancel",
            "Отмена",
            "Annuler",
            "Abbrechen",
            "Cancelar",
            "Annulla",
        ],
    ),
    (
        "cancelled",
        [
            "Stopped. The best verified result is kept when available.",
            "Остановлено. Лучший проверенный результат сохранён, если он получен.",
            "Arrêté. Le meilleur résultat vérifié disponible est conservé.",
            "Gestoppt. Das beste geprüfte Ergebnis wird beibehalten, falls vorhanden.",
            "Detenido. Se conserva el mejor resultado verificado disponible.",
            "Fermato. Il miglior risultato verificato disponibile viene conservato.",
        ],
    ),
    (
        "empty_plan",
        [
            "Your cutting plan will appear here.",
            "Здесь появится план раскроя.",
            "Votre plan apparaîtra ici.",
            "Ihr Zuschnittplan erscheint hier.",
            "Aquí aparecerá el plan de corte.",
            "Il piano di taglio apparirà qui.",
        ],
    ),
    (
        "heuristic",
        [
            "Verified guillotine layout · heuristic, not guaranteed optimal",
            "Проверенный гильотинный раскрой · эвристика без гарантии оптимальности",
            "Plan guillotine vérifié · heuristique sans garantie d’optimalité",
            "Geprüfter Guillotine-Zuschnitt · heuristisch, ohne Optimalitätsgarantie",
            "Plan guillotina verificado · heurística sin garantía de óptimo",
            "Taglio a ghigliottina verificato · euristica senza garanzia di ottimalità",
        ],
    ),
    (
        "sheets_used",
        [
            "Sheets used",
            "Листов использовано",
            "Panneaux utilisés",
            "Verwendete Platten",
            "Tableros usados",
            "Pannelli usati",
        ],
    ),
    (
        "placed",
        [
            "Parts placed",
            "Деталей размещено",
            "Pièces placées",
            "Platzierte Teile",
            "Piezas colocadas",
            "Pezzi posizionati",
        ],
    ),
    (
        "utilization",
        [
            "Material utilization",
            "Использование материала",
            "Utilisation de matière",
            "Materialausnutzung",
            "Aprovechamiento",
            "Utilizzo del materiale",
        ],
    ),
    (
        "waste",
        [
            "Unused material (incl. kerf)",
            "Остаток и потери на пропил",
            "Matière inutilisée (trait inclus)",
            "Restmaterial (inkl. Schnittfuge)",
            "Material sin usar (incluye corte)",
            "Materiale inutilizzato (taglio incluso)",
        ],
    ),
    (
        "cuts",
        [
            "Cut sequence",
            "Порядок резов",
            "Séquence de coupe",
            "Schnittfolge",
            "Secuencia de cortes",
            "Sequenza di taglio",
        ],
    ),
    (
        "offcuts",
        [
            "Reusable offcuts",
            "Полезные остатки",
            "Chutes réutilisables",
            "Nutzbare Reststücke",
            "Sobrantes reutilizables",
            "Sfridi riutilizzabili",
        ],
    ),
    (
        "unplaced",
        [
            "Unplaced parts",
            "Неразмещённые детали",
            "Pièces non placées",
            "Nicht platzierte Teile",
            "Piezas sin colocar",
            "Pezzi non posizionati",
        ],
    ),
    (
        "sheet",
        ["Sheet", "Лист", "Panneau", "Platte", "Tablero", "Pannello"],
    ),
    (
        "step",
        ["Step", "Шаг", "Étape", "Schritt", "Paso", "Passaggio"],
    ),
    (
        "vertical",
        [
            "Vertical",
            "Вертикальный",
            "Verticale",
            "Senkrecht",
            "Vertical",
            "Verticale",
        ],
    ),
    (
        "horizontal",
        [
            "Horizontal",
            "Горизонтальный",
            "Horizontale",
            "Waagerecht",
            "Horizontal",
            "Orizzontale",
        ],
    ),
    (
        "trim_cut",
        [
            "Edge trim",
            "Обрезка края",
            "Délignage du bord",
            "Kantenbeschnitt",
            "Recorte del borde",
            "Rifilatura del bordo",
        ],
    ),
    (
        "separate_cut",
        [
            "Separate",
            "Разделение",
            "Séparation",
            "Trennen",
            "Separación",
            "Separazione",
        ],
    ),
    (
        "offset",
        [
            "From left / top of this piece",
            "От левого / верхнего края заготовки",
            "Depuis la gauche / le haut de cette pièce",
            "Ab linker / oberer Kante dieses Stücks",
            "Desde la izquierda / arriba de esta pieza",
            "Dal bordo sinistro / superiore di questo pezzo",
        ],
    ),
    (
        "workpiece",
        [
            "Workpiece",
            "Заготовка",
            "Pièce de travail",
            "Werkstück",
            "Pieza de trabajo",
            "Pezzo in lavorazione",
        ],
    ),
    (
        "origin",
        [
            "Origin on sheet (x, y)",
            "Начало на листе (x, y)",
            "Origine sur le panneau (x, y)",
            "Ursprung auf Platte (x, y)",
            "Origen en tablero (x, y)",
            "Origine sul pannello (x, y)",
        ],
    ),
    (
        "reason",
        ["Reason", "Причина", "Motif", "Grund", "Motivo", "Motivo"],
    ),
    (
        "reason_size",
        [
            "Size / rotation / full kerf cannot fit",
            "Не помещается с учётом размера, поворота и пропила",
            "Taille / rotation / trait complet incompatibles",
            "Größe / Drehung / volle Schnittfuge passen nicht",
            "No cabe por tamaño, giro o ancho de corte",
            "Non entra per misura, rotazione o spessore del taglio",
        ],
    ),
    (
        "reason_grain",
        [
            "No compatible sheet grain direction",
            "Нет листа с подходящим направлением волокон",
            "Aucun fil de panneau compatible",
            "Keine passende Faserrichtung der Platte",
            "Sin tablero con veta compatible",
            "Nessun pannello con venatura compatibile",
        ],
    ),
    (
        "reason_stock",
        [
            "No remaining feasible space in available stock",
            "Нет подходящего места в доступном количестве листов",
            "Plus d’espace réalisable dans les panneaux disponibles",
            "Kein passender Platz im verfügbaren Bestand",
            "Sin espacio viable en los tableros disponibles",
            "Spazio utilizzabile esaurito nei pannelli disponibili",
        ],
    ),
    (
        "reason_cancelled",
        [
            "Calculation was stopped",
            "Расчёт остановлен",
            "Calcul arrêté",
            "Berechnung gestoppt",
            "Cálculo detenido",
            "Calcolo fermato",
        ],
    ),
    (
        "fit",
        [
            "Fit sheet",
            "Вписать лист",
            "Ajuster le panneau",
            "Platte einpassen",
            "Ajustar tablero",
            "Adatta pannello",
        ],
    ),
    (
        "canvas_help",
        [
            "Wheel: zoom · Drag: pan · Click a part or cut to inspect",
            "Колесо: масштаб · Перетаскивание: сдвиг · Нажмите деталь или рез",
            "Molette : zoom · Glisser : déplacer · Cliquer pour inspecter",
            "Mausrad: Zoom · Ziehen: Verschieben · Teil oder Schnitt anklicken",
            "Rueda: zoom · Arrastrar: desplazar · Clic para inspeccionar",
            "Rotella: zoom · Trascina: sposta · Clic per ispezionare",
        ],
    ),
    (
        "legend",
        [
            "Parts · Offcuts · Orange: selected cut · Arrows: grain",
            "Детали · Остатки · Оранжевый: выбранный рез · Стрелки: волокна",
            "Pièces · Chutes · Orange : coupe choisie · Flèches : fil",
            "Teile · Reste · Orange: gewählter Schnitt · Pfeile: Faser",
            "Piezas · Sobrantes · Naranja: corte seleccionado · Flechas: veta",
            "Pezzi · Sfridi · Arancione: taglio scelto · Frecce: venatura",
        ],
    ),
    (
        "export_report",
        [
            "Export printable report…",
            "Экспорт отчёта для печати…",
            "Exporter le rapport imprimable…",
            "Druckbericht exportieren…",
            "Exportar informe imprimible…",
            "Esporta rapporto stampabile…",
        ],
    ),
    (
        "export_svg",
        [
            "Export sheet SVG…",
            "Экспорт листа в SVG…",
            "Exporter le panneau SVG…",
            "Platte als SVG exportieren…",
            "Exportar tablero SVG…",
            "Esporta pannello SVG…",
        ],
    ),
    (
        "import_csv",
        [
            "Import parts CSV…",
            "Импорт деталей из CSV…",
            "Importer des pièces CSV…",
            "Teile aus CSV importieren…",
            "Importar piezas CSV…",
            "Importa pezzi CSV…",
        ],
    ),
    (
        "export_csv",
        [
            "Export parts CSV…",
            "Экспорт деталей в CSV…",
            "Exporter les pièces CSV…",
            "Teile als CSV exportieren…",
            "Exportar piezas CSV…",
            "Esporta pezzi CSV…",
        ],
    ),
    (
        "csv_help",
        [
            "CSV: name,width,height,quantity,rotate,grain. Use current units, true/false, none/width/height. Import replaces the parts list.",
            "CSV: name,width,height,quantity,rotate,grain. Текущие единицы; true/false; none/width/height. Импорт заменяет список деталей.",
            "CSV : name,width,height,quantity,rotate,grain. Unités actuelles ; true/false ; none/width/height. L’import remplace les pièces.",
            "CSV: name,width,height,quantity,rotate,grain. Aktuelle Einheiten; true/false; none/width/height. Import ersetzt die Teileliste.",
            "CSV: name,width,height,quantity,rotate,grain. Unidades actuales; true/false; none/width/height. Reemplaza la lista de piezas.",
            "CSV: name,width,height,quantity,rotate,grain. Unità attuali; true/false; none/width/height. L’importazione sostituisce i pezzi.",
        ],
    ),
    (
        "language",
        ["Language", "Язык", "Langue", "Sprache", "Idioma", "Lingua"],
    ),
    (
        "appearance",
        [
            "Appearance",
            "Оформление",
            "Apparence",
            "Darstellung",
            "Apariencia",
            "Aspetto",
        ],
    ),
    (
        "light",
        ["Light", "Светлая", "Clair", "Hell", "Claro", "Chiaro"],
    ),
    (
        "dark",
        ["Dark", "Тёмная", "Sombre", "Dunkel", "Oscuro", "Scuro"],
    ),
    (
        "reduced_motion",
        [
            "Reduce motion",
            "Уменьшить анимацию",
            "Réduire les animations",
            "Bewegung reduzieren",
            "Reducir movimiento",
            "Riduci animazioni",
        ],
    ),
    (
        "updates",
        [
            "Updates",
            "Обновления",
            "Mises à jour",
            "Updates",
            "Actualizaciones",
            "Aggiornamenti",
        ],
    ),
    (
        "installed_version",
        [
            "Installed version",
            "Установленная версия",
            "Version installée",
            "Installierte Version",
            "Versión instalada",
            "Versione installata",
        ],
    ),
    (
        "startup_check",
        [
            "Check for updates on startup",
            "Проверять обновления при запуске",
            "Vérifier les mises à jour au démarrage",
            "Beim Start nach Updates suchen",
            "Buscar actualizaciones al iniciar",
            "Cerca aggiornamenti all’avvio",
        ],
    ),
    (
        "check_updates",
        [
            "Check for updates",
            "Проверить обновления",
            "Rechercher des mises à jour",
            "Nach Updates suchen",
            "Buscar actualizaciones",
            "Cerca aggiornamenti",
        ],
    ),
    (
        "checking",
        [
            "Checking…",
            "Проверка…",
            "Vérification…",
            "Prüfen…",
            "Buscando…",
            "Ricerca…",
        ],
    ),
    (
        "up_to_date",
        [
            "You have the latest stable version.",
            "Установлена последняя стабильная версия.",
            "Vous avez la dernière version stable.",
            "Sie haben die aktuelle stabile Version.",
            "Tienes la última versión estable.",
            "Hai l’ultima versione stabile.",
        ],
    ),
    (
        "update_available",
        [
            "A new version is available",
            "Доступна новая версия",
            "Une nouvelle version est disponible",
            "Eine neue Version ist verfügbar",
            "Hay una nueva versión",
            "È disponibile una nuova versione",
        ],
    ),
    (
        "install_update",
        [
            "Download and install",
            "Скачать и установить",
            "Télécharger et installer",
            "Herunterladen und installieren",
            "Descargar e instalar",
            "Scarica e installa",
        ],
    ),
    (
        "downloading",
        [
            "Downloading and verifying…",
            "Загрузка и проверка…",
            "Téléchargement et vérification…",
            "Herunterladen und prüfen…",
            "Descargando y verificando…",
            "Download e verifica…",
        ],
    ),
    (
        "restart_update",
        [
            "Restart to install",
            "Перезапустить и установить",
            "Redémarrer pour installer",
            "Zum Installieren neu starten",
            "Reiniciar para instalar",
            "Riavvia per installare",
        ],
    ),
    (
        "update_ready",
        [
            "Verified update ready. Save your work before restarting.",
            "Обновление проверено. Сохраните проект перед перезапуском.",
            "Mise à jour vérifiée. Enregistrez avant de redémarrer.",
            "Update geprüft. Speichern Sie vor dem Neustart.",
            "Actualización verificada. Guarda antes de reiniciar.",
            "Aggiornamento verificato. Salva prima di riavviare.",
        ],
    ),
    (
        "update_note",
        [
            "Only checking is automatic. Downloads and installation require your action.",
            "Автоматически выполняется только проверка. Загрузку и установку запускаете вы.",
            "Seule la vérification est automatique. Téléchargement et installation sont manuels.",
            "Nur die Prüfung ist automatisch. Download und Installation starten Sie selbst.",
            "Solo la búsqueda es automática. Tú inicias la descarga e instalación.",
            "Solo la ricerca è automatica. Download e installazione richiedono la tua azione.",
        ],
    ),
    (
        "update_failed",
        [
            "Could not check or install the update.",
            "Не удалось проверить или установить обновление.",
            "Impossible de vérifier ou d’installer la mise à jour.",
            "Update konnte nicht geprüft oder installiert werden.",
            "No se pudo buscar o instalar la actualización.",
            "Impossibile cercare o installare l’aggiornamento.",
        ],
    ),
    (
        "update_unpublished",
        [
            "The repository or a published release is not available yet.",
            "Репозиторий или опубликованный релиз пока недоступен.",
            "Le dépôt ou une version publiée n’est pas encore disponible.",
            "Repository oder veröffentlichte Version noch nicht verfügbar.",
            "El repositorio o una versión publicada aún no está disponible.",
            "Repository o versione pubblicata non ancora disponibili.",
        ],
    ),
    (
        "update_rate_limit",
        [
            "GitHub request limit reached. Try again later.",
            "Достигнут лимит запросов GitHub. Попробуйте позже.",
            "Limite de requêtes GitHub atteinte. Réessayez plus tard.",
            "GitHub-Anfragelimit erreicht. Später erneut versuchen.",
            "Límite de GitHub alcanzado. Inténtalo más tarde.",
            "Limite di richieste GitHub raggiunto. Riprova più tardi.",
        ],
    ),
    (
        "update_asset_missing",
        [
            "No compatible verified Windows executable in this release.",
            "В релизе нет совместимого проверяемого Windows EXE.",
            "Aucun exécutable Windows compatible et vérifiable dans cette version.",
            "Keine kompatible prüfbare Windows-Datei in dieser Version.",
            "La versión no incluye un EXE de Windows compatible y verificable.",
            "La versione non include un EXE Windows compatibile e verificabile.",
        ],
    ),
    (
        "update_verification",
        [
            "Download verification failed. The installed app was preserved.",
            "Проверка загрузки не пройдена. Установленное приложение сохранено.",
            "Échec de vérification. L’application installée est conservée.",
            "Downloadprüfung fehlgeschlagen. Installierte App bleibt erhalten.",
            "Falló la verificación. Se conservó la aplicación instalada.",
            "Verifica fallita. L’app installata è stata conservata.",
        ],
    ),
    (
        "operation_failed",
        [
            "Could not complete the action",
            "Не удалось выполнить действие",
            "Impossible de terminer l’action",
            "Aktion konnte nicht abgeschlossen werden",
            "No se pudo completar la acción",
            "Impossibile completare l’azione",
        ],
    ),
    (
        "saved",
        [
            "Saved",
            "Сохранено",
            "Enregistré",
            "Gespeichert",
            "Guardado",
            "Salvato",
        ],
    ),
    (
        "exported",
        [
            "Exported",
            "Экспортировано",
            "Exporté",
            "Exportiert",
            "Exportado",
            "Esportato",
        ],
    ),
    (
        "unsaved",
        [
            "Unsaved changes",
            "Есть несохранённые изменения",
            "Modifications non enregistrées",
            "Ungespeicherte Änderungen",
            "Cambios sin guardar",
            "Modifiche non salvate",
        ],
    ),
    (
        "discard",
        [
            "Discard changes",
            "Не сохранять",
            "Ignorer les modifications",
            "Änderungen verwerfen",
            "Descartar cambios",
            "Scarta modifiche",
        ],
    ),
    (
        "keep_editing",
        [
            "Keep editing",
            "Продолжить работу",
            "Continuer à modifier",
            "Weiter bearbeiten",
            "Seguir editando",
            "Continua a modificare",
        ],
    ),
    (
        "unsaved_help",
        [
            "Save this project before continuing?",
            "Сохранить проект перед продолжением?",
            "Enregistrer ce projet avant de continuer ?",
            "Projekt vor dem Fortfahren speichern?",
            "¿Guardar el proyecto antes de continuar?",
            "Salvare il progetto prima di continuare?",
        ],
    ),
    (
        "close",
        [
            "Close",
            "Закрыть",
            "Fermer",
            "Schließen",
            "Cerrar",
            "Chiudi",
        ],
    ),
    (
        "none",
        ["None", "Нет", "Aucun", "Keine", "Ninguno", "Nessuno"],
    ),
    (
        "number_required",
        [
            "Enter a number.",
            "Введите число.",
            "Saisissez un nombre.",
            "Zahl eingeben.",
            "Introduce un número.",
            "Inserisci un numero.",
        ],
    ),
    (
        "invalid_number",
        [
            "Use a positive decimal number.",
            "Нужно положительное десятичное число.",
            "Utilisez un nombre décimal positif.",
            "Positive Dezimalzahl verwenden.",
            "Usa un número decimal positivo.",
            "Usa un numero decimale positivo.",
        ],
    ),
    (
        "precision_limit",
        [
            "Use at most four decimal places.",
            "Не более четырёх знаков после запятой.",
            "Quatre décimales au maximum.",
            "Höchstens vier Nachkommastellen.",
            "Máximo cuatro decimales.",
            "Massimo quattro decimali.",
        ],
    ),
    (
        "size_limit",
        [
            "Each dimension must be at most 100 metres.",
            "Каждый размер — не больше 100 метров.",
            "Chaque dimension est limitée à 100 mètres.",
            "Jede Abmessung höchstens 100 Meter.",
            "Cada medida debe ser de hasta 100 metros.",
            "Ogni dimensione deve essere al massimo 100 metri.",
        ],
    ),
    (
        "positive_size",
        [
            "Dimensions must be greater than zero.",
            "Размеры должны быть больше нуля.",
            "Les dimensions doivent être positives.",
            "Abmessungen müssen größer als null sein.",
            "Las medidas deben ser mayores que cero.",
            "Le dimensioni devono essere maggiori di zero.",
        ],
    ),
    (
        "positive_quantity",
        [
            "Enter a whole quantity greater than zero.",
            "Введите целое количество больше нуля.",
            "Saisissez une quantité entière positive.",
            "Positive ganze Anzahl eingeben.",
            "Introduce una cantidad entera positiva.",
            "Inserisci una quantità intera positiva.",
        ],
    ),
    (
        "name_required",
        [
            "Give each sheet and part a name.",
            "Задайте название каждого листа и детали.",
            "Nommez chaque panneau et pièce.",
            "Jede Platte und jedes Teil benennen.",
            "Asigna nombre a cada tablero y pieza.",
            "Assegna un nome a ogni pannello e pezzo.",
        ],
    ),
    (
        "name_limit",
        [
            "Names must fit within 256 UTF-8 bytes.",
            "Названия должны занимать не более 256 байт UTF-8.",
            "Les noms sont limités à 256 octets UTF-8.",
            "Namen dürfen höchstens 256 UTF-8-Bytes belegen.",
            "Los nombres deben caber en 256 bytes UTF-8.",
            "I nomi devono rientrare in 256 byte UTF-8.",
        ],
    ),
    (
        "row_limit",
        [
            "Use at most 256 rows per list.",
            "Не более 256 строк в каждом списке.",
            "256 lignes au maximum par liste.",
            "Höchstens 256 Zeilen je Liste.",
            "Máximo 256 filas por lista.",
            "Massimo 256 righe per elenco.",
        ],
    ),
    (
        "part_limit",
        [
            "Use at most 2,000 required parts per project.",
            "Не более 2000 деталей в проекте.",
            "2 000 pièces au maximum par projet.",
            "Höchstens 2.000 Teile je Projekt.",
            "Máximo 2000 piezas por proyecto.",
            "Massimo 2000 pezzi per progetto.",
        ],
    ),
    (
        "sheet_limit",
        [
            "Use at most 256 available sheets per project.",
            "Не более 256 доступных листов в проекте.",
            "256 panneaux disponibles au maximum.",
            "Höchstens 256 verfügbare Platten je Projekt.",
            "Máximo 256 tableros disponibles.",
            "Massimo 256 pannelli disponibili.",
        ],
    ),
    (
        "trim_kerf",
        [
            "Edge trim must be zero or at least the saw kerf.",
            "Обрезка края должна быть нулевой или не меньше пропила.",
            "Le délignage doit être nul ou au moins égal au trait.",
            "Beschnitt muss null oder mindestens so breit wie die Fuge sein.",
            "El recorte debe ser cero o al menos igual al ancho de corte.",
            "La rifilatura deve essere zero o almeno pari al taglio.",
        ],
    ),
    (
        "trim_size",
        [
            "Edge trim leaves no usable sheet.",
            "После обрезки краёв лист не остаётся.",
            "Le délignage ne laisse aucun panneau utile.",
            "Nach Beschnitt bleibt keine nutzbare Platte.",
            "El recorte no deja tablero útil.",
            "La rifilatura non lascia pannello utilizzabile.",
        ],
    ),
    (
        "invalid_constraints",
        [
            "Cut settings must be nonnegative dimensions.",
            "Параметры реза должны быть неотрицательными размерами.",
            "Les réglages doivent être des dimensions positives ou nulles.",
            "Schnittmaße dürfen nicht negativ sein.",
            "Los ajustes deben ser medidas no negativas.",
            "Le impostazioni devono essere misure non negative.",
        ],
    ),
    (
        "duplicate_id",
        [
            "The project contains duplicate row identifiers.",
            "В проекте повторяются идентификаторы строк.",
            "Le projet contient des identifiants en double.",
            "Projekt enthält doppelte Zeilenkennungen.",
            "El proyecto tiene identificadores duplicados.",
            "Il progetto contiene identificatori duplicati.",
        ],
    ),
    (
        "stock_and_parts_required",
        [
            "Add at least one sheet and one part.",
            "Добавьте хотя бы один лист и одну деталь.",
            "Ajoutez au moins un panneau et une pièce.",
            "Mindestens eine Platte und ein Teil hinzufügen.",
            "Añade al menos un tablero y una pieza.",
            "Aggiungi almeno un pannello e un pezzo.",
        ],
    ),
    (
        "file_size_limit",
        [
            "This file exceeds the 8 MB limit.",
            "Размер файла превышает 8 МБ.",
            "Ce fichier dépasse la limite de 8 Mo.",
            "Datei überschreitet die Grenze von 8 MB.",
            "El archivo supera el límite de 8 MB.",
            "Il file supera il limite di 8 MB.",
        ],
    ),
    (
        "unsupported_project_version",
        [
            "This project requires a newer version of Cut Planner.",
            "Для проекта нужна более новая версия Cut Planner.",
            "Ce projet exige une version plus récente de Cut Planner.",
            "Projekt benötigt eine neuere Version von Cut Planner.",
            "Este proyecto requiere una versión más reciente de Cut Planner.",
            "Il progetto richiede una versione più recente di Cut Planner.",
        ],
    ),
    (
        "csv_headers",
        [
            "CSV headers must be: name,width,height,quantity,rotate,grain",
            "Заголовки CSV: name,width,height,quantity,rotate,grain",
            "En-têtes CSV : name,width,height,quantity,rotate,grain",
            "CSV-Kopfzeile: name,width,height,quantity,rotate,grain",
            "Cabecera CSV: name,width,height,quantity,rotate,grain",
            "Intestazioni CSV: name,width,height,quantity,rotate,grain",
        ],
    ),
    (
        "csv_empty",
        [
            "The CSV file is empty.",
            "Файл CSV пуст.",
            "Le fichier CSV est vide.",
            "CSV-Datei ist leer.",
            "El CSV está vacío.",
            "Il file CSV è vuoto.",
        ],
    ),
    (
        "csv_columns",
        [
            "CSV columns must match the header.",
            "Число столбцов CSV должно совпадать с заголовком.",
            "Les colonnes CSV doivent correspondre à l’en-tête.",
            "CSV-Spalten müssen zur Kopfzeile passen.",
            "Las columnas CSV deben coincidir con la cabecera.",
            "Le colonne CSV devono corrispondere all’intestazione.",
        ],
    ),
    (
        "csv_grain",
        [
            "CSV grain must be none, width or height.",
            "В CSV направление: none, width или height.",
            "Le fil CSV doit être none, width ou height.",
            "CSV-Faser muss none, width oder height sein.",
            "La veta CSV debe ser none, width o height.",
            "La venatura CSV deve essere none, width o height.",
        ],
    ),
    (
        "csv_boolean",
        [
            "CSV rotation must be true or false.",
            "В CSV поворот: true или false.",
            "La rotation CSV doit être true ou false.",
            "CSV-Drehung muss true oder false sein.",
            "El giro CSV debe ser true o false.",
            "La rotazione CSV deve essere true o false.",
        ],
    ),
    (
        "csv_quotes",
        [
            "The CSV file contains invalid quotation marks.",
            "Неправильные кавычки в CSV.",
            "Le CSV contient des guillemets incorrects.",
            "CSV-Datei enthält ungültige Anführungszeichen.",
            "El CSV contiene comillas incorrectas.",
            "Il CSV contiene virgolette non valide.",
        ],
    ),
    (
        "report_note",
        [
            "Dimensions are finished part sizes. Cuts span the full current workpiece. Offsets are measured to the near edge of the kerf; keep the blade on the waste side. Check material, blade, clamps and saw capacity before cutting. The plan does not model saw-specific safety clearances.",
            "Указаны готовые размеры деталей. Рез проходит через всю текущую заготовку. Отступ — до ближнего края пропила; пила идёт со стороны отхода. Перед работой проверьте материал, пилу, зажимы и возможности станка. Зазоры безопасности конкретного станка не моделируются.",
            "Les cotes sont celles des pièces finies. Les coupes traversent la pièce entière. Le décalage atteint le bord proche du trait ; lame côté chute. Vérifiez matière, lame, serrage et capacité de la scie. Les dégagements de sécurité propres à la scie ne sont pas modélisés.",
            "Maße sind Fertigmaße. Schnitte verlaufen durch das ganze Werkstück. Abstand bis zur nahen Fugenkante; Blatt auf Restseite führen. Material, Blatt, Klemmen und Sägekapazität prüfen. Sägespezifische Sicherheitsabstände sind nicht modelliert.",
            "Las medidas son finales. Cada corte cruza toda la pieza actual. La distancia llega al borde cercano del corte; coloca la hoja en el sobrante. Revisa material, hoja, sujeción y capacidad de la sierra. No se modelan holguras de seguridad específicas.",
            "Le misure sono quelle finali. I tagli attraversano tutto il pezzo corrente. La distanza arriva al bordo vicino del taglio; lama sullo sfrido. Controlla materiale, lama, morsetti e capacità della sega. Le distanze di sicurezza specifiche non sono modellate.",
        ],
    ),
    (
        "unit_note",
        [
            "Changing display units preserves exact geometry. Converted labels may be rounded to six decimals.",
            "Смена единиц сохраняет точную геометрию. Переведённые размеры могут округляться до шести знаков.",
            "Changer d’unité conserve la géométrie exacte. Les cotes converties peuvent être arrondies à six décimales.",
            "Einheitenwechsel erhält die exakte Geometrie. Umgerechnete Werte können auf sechs Stellen gerundet sein.",
            "Cambiar unidades conserva la geometría exacta. Las etiquetas convertidas pueden redondearse a seis decimales.",
            "Cambiare unità conserva la geometria esatta. Le misure convertite possono essere arrotondate a sei decimali.",
        ],
    ),
    ("row", ["Row", "Строка", "Ligne", "Zeile", "Fila", "Riga"]),
    (
        "validating",
        [
            "The generated plan failed independent validation. No export was produced.",
            "План не прошёл независимую проверку. Экспорт не создан.",
            "Le plan a échoué à la validation indépendante. Aucun export créé.",
            "Plan hat unabhängige Prüfung nicht bestanden. Kein Export erstellt.",
            "El plan no superó la validación independiente. No se generó exportación.",
            "Il piano non ha superato la verifica indipendente. Nessuna esportazione creata.",
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogs_are_complete_unique_and_have_english_fallback() {
        let mut keys = std::collections::HashSet::new();
        for (key, values) in MESSAGES {
            assert!(keys.insert(key), "duplicate {key}");
            for value in values {
                assert!(!value.trim().is_empty(), "missing translation: {key}");
            }
        }
        assert_eq!(tr("unknown", "save"), "Save");
        assert_eq!(tr("ru", "save"), "Сохранить");
    }
}
