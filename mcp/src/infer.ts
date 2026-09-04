import {
  BOOLEAN_TYPES,
  CARDINALITY_ATTRS,
  DATETIME_TYPES,
  DEPENDENCY_ATTRS,
  IDENTITY_FIELDS,
  INTEGER_TYPES,
  NUMERIC_TYPES,
  STRING_TYPES,
  SYNC_SENSITIVE,
  TYPE_ALIASES,
  UI_WIDGETS,
} from "./standards.js";

export type CatalogAttr = {
  name: string;
  args: unknown[];
};

export type CatalogField = {
  name: string;
  type?: string | null;
  attributes: CatalogAttr[];
};

export type CatalogModel = {
  name: string;
  traits: string[];
  fields: CatalogField[];
};

export type Catalog = {
  ok?: boolean;
  models: CatalogModel[];
  traits?: string[];
  types?: string[];
  attributes?: string[];
  fields?: string[];
};

export type KeywordRow = {
  keyword: string;
  kind: string;
  platforms: string[];
};

export type Suggestion = {
  target: string;
  suggest: string;
  reason: string;
};

export type InferResult = {
  trait_suggestions: Suggestion[];
  attribute_suggestions: Suggestion[];
  association_suggestions: Suggestion[];
  sync_risks: Suggestion[];
  standard_renames: Suggestion[];
};

export function inferPatterns(catalog: Catalog, keywords: KeywordRow[]): InferResult {
  return {
    trait_suggestions: traitSuggestions(catalog),
    attribute_suggestions: attributeSuggestions(catalog),
    association_suggestions: associationSuggestions(catalog),
    sync_risks: syncRisks(catalog, keywords),
    standard_renames: standardRenames(catalog),
  };
}

const UNUSUAL =
  "Unusual for the default MCP vocabulary; the compiler still accepts it. Review when writing or reviewing .dtct / .ngin, or ignore if this project means it.";

export function parseKeywordsTable(text: string): KeywordRow[] {
  const rows: KeywordRow[] = [];
  let seenHeader = false;
  let skipSep = false;
  for (const line of text.split(/\r?\n/)) {
    const cells = tableCells(line);
    if (!cells) {
      continue;
    } else if (!seenHeader) {
      if (isKeywordsHeader(cells)) {
        seenHeader = true;
        skipSep = true;
      }
    } else if (skipSep && isTableSep(cells)) {
      skipSep = false;
    } else if (cells.length === 5) {
      skipSep = false;
      const platforms = cells[4]
        .split(",")
        .map((item) => item.trim())
        .filter((item) => item.length > 0);
      if (cells[0].length > 0) {
        rows.push({ keyword: cells[0], kind: cells[1], platforms });
      }
    }
  }
  return rows;
}

function traitSuggestions(catalog: Catalog): Suggestion[] {
  const out: Suggestion[] = [];
  const byShape = new Map<string, string[]>();
  for (const model of catalog.models) {
    if (model.fields.length === 0) {
      if (!hasTrait(model, "Enum") && !hasTrait(model, "Data")) {
        out.push({
          target: model.name,
          suggest: "Enum",
          reason: "No fields — treat as Enum so every future consumer shares the same discriminants.",
        });
      }
    } else {
      if (/(Record|Data)$/.test(model.name) && !hasTrait(model, "Data") && !hasTrait(model, "Enum")) {
        out.push({
          target: model.name,
          suggest: "Data",
          reason: "Name looks like an encapsulated record; tag Data now even if only one platform exists.",
        });
      }
      if (model.fields.length >= 2) {
        const key = model.fields
          .map((field) => field.name)
          .sort()
          .join(",");
        const group = byShape.get(key) ?? [];
        group.push(model.name);
        byShape.set(key, group);
      }
    }
  }
  for (const names of byShape.values()) {
    if (names.length >= 2) {
      for (const name of names) {
        const model = catalog.models.find((item) => item.name === name);
        if (model && !hasTrait(model, "Data") && !hasTrait(model, "Enum")) {
          out.push({
            target: name,
            suggest: "Data",
            reason: "Same field set as another model — one Data trait across platforms, not a per-platform trait.",
          });
        }
      }
    }
  }
  return dedupe(out);
}

function attributeSuggestions(catalog: Catalog): Suggestion[] {
  const out: Suggestion[] = [];
  for (const model of catalog.models) {
    for (const field of model.fields) {
      const target = `${model.name}.${field.name}`;
      const ty = field.type ?? "";
      if (STRING_TYPES.has(ty) && /email/i.test(field.name) && !hasAttr(field, "email")) {
        out.push({
          target,
          suggest: "email",
          reason: "String field name looks like an email; keep the same attribute on every platform.",
        });
      }
      if (/(phone_no|phone|mobile)/i.test(field.name) && !hasAttr(field, "phone_no")) {
        out.push({
          target,
          suggest: "phone_no",
          reason: "Field name looks like a phone number.",
        });
      }
      if (IDENTITY_FIELDS.has(field.name.toLowerCase()) && !hasAttr(field, "unique")) {
        out.push({
          target,
          suggest: "unique",
          reason: "Identity-like field; uniqueness must stay consistent if another service is added.",
        });
      }
      if (/(optional|_opt$|^opt_)/i.test(field.name) && !hasAttr(field, "nullable")) {
        out.push({
          target,
          suggest: "nullable",
          reason: "Name looks optional; nullable belongs on the contract, not only in one backend.",
        });
      }
      if (INTEGER_TYPES.has(ty) && /unsigned/i.test(field.name) && !hasAttr(field, "unsigned")) {
        out.push({
          target,
          suggest: "unsigned",
          reason: "Integer width should use unsigned as an attribute (i32<unsigned>), not a separate type.",
        });
      }
      suggestWidget(out, target, field, ty);
      suggestRelationship(out, catalog, target, field, ty);
    }
  }
  return out;
}

function suggestWidget(out: Suggestion[], target: string, field: CatalogField, ty: string): void {
  if (attrsIn(field, UI_WIDGETS).length > 0) {
    return;
  } else if (ty === "relationship") {
    if (/(search|lookup|remote)/i.test(field.name)) {
      out.push({
        target,
        suggest: "AsyncSelect",
        reason: "Relationship name looks remote; AsyncSelect is a default widget suggestion, not a type.",
      });
    } else {
      out.push({
        target,
        suggest: "Select",
        reason: "Type is not the widget; relationship fields often use Select. Pick Radio/AsyncSelect if this project prefers that.",
      });
    }
  } else if (BOOLEAN_TYPES.has(ty)) {
    out.push({
      target,
      suggest: "Checkbox",
      reason: "Boolean fields often use Checkbox. This is a default suggestion, not a language rule.",
    });
  } else if (DATETIME_TYPES.has(ty)) {
    if (/(_date$|^date_|birthday)/i.test(field.name)) {
      out.push({
        target,
        suggest: "Date",
        reason: "Name looks date-only; Date is a widget, datetime is the type.",
      });
    } else {
      out.push({
        target,
        suggest: "Datetime",
        reason: "datetime type often uses a Datetime widget; override with Date or DatetimeRange if needed.",
      });
    }
  } else if (NUMERIC_TYPES.has(ty) && hasAttr(field, "min") && hasAttr(field, "max")) {
    out.push({
      target,
      suggest: "Range",
      reason: "Numeric min and max often map to a Range widget.",
    });
  } else if (/(file|image|upload|photo|avatar)/i.test(field.name)) {
    out.push({
      target,
      suggest: "Upload",
      reason: "Name looks like a file; Upload is a widget suggestion, not implied by string.",
    });
  } else if (STRING_TYPES.has(ty) && /^(status|kind|role|type)$/i.test(field.name)) {
    out.push({
      target,
      suggest: "Select",
      reason: "String is not always Text; status/kind/role/type often use Select.",
    });
  }
}

function suggestRelationship(
  out: Suggestion[],
  catalog: Catalog,
  target: string,
  field: CatalogField,
  ty: string,
): void {
  if (ty !== "relationship") {
    return;
  } else {
    if (attrsIn(field, CARDINALITY_ATTRS).length === 0) {
      const matched = modelNamed(catalog, field.name);
      if (matched) {
        out.push({
          target,
          suggest: "BelongsTo",
          reason: `Singular name matches model ${matched}; BelongsTo is a default cardinality suggestion.`,
        });
      } else if (field.name.endsWith("s") && field.name.length > 1) {
        out.push({
          target,
          suggest: "OneToMany",
          reason: "Plural name; OneToMany is a default cardinality suggestion.",
        });
      }
    }
    if (attrsIn(field, DEPENDENCY_ATTRS).length === 0) {
      if (hasAttr(field, "required")) {
        out.push({
          target,
          suggest: "Full",
          reason: "required relationship often uses Full dependency in the default vocabulary.",
        });
      } else if (hasAttr(field, "nullable")) {
        out.push({
          target,
          suggest: "Partial",
          reason: "nullable relationship often uses Partial dependency in the default vocabulary.",
        });
      }
    }
    if (!hasAttr(field, "model")) {
      const matched = modelNamed(catalog, field.name);
      if (matched) {
        out.push({
          target,
          suggest: "model",
          reason: `Field name matches model ${matched}; add model(${matched}). The compiler does not require it.`,
        });
      }
    }
  }
}

function associationSuggestions(catalog: Catalog): Suggestion[] {
  const out: Suggestion[] = [];
  for (const model of catalog.models) {
    for (const field of model.fields) {
      const target = `${model.name}.${field.name}`;
      const ty = field.type ?? "";
      const cards = attrsIn(field, CARDINALITY_ATTRS);
      const deps = attrsIn(field, DEPENDENCY_ATTRS);
      const widgets = attrsIn(field, UI_WIDGETS);
      if (cards.length >= 2) {
        out.push({
          target,
          suggest: "keep a single cardinality flag",
          reason: `${cards.join(" and ")} on one field. ${UNUSUAL}`,
        });
      }
      if (deps.length >= 2) {
        out.push({
          target,
          suggest: "keep a single dependency flag",
          reason: `${deps.join(" and ")} on one field. ${UNUSUAL}`,
        });
      }
      if (widgets.length >= 2) {
        out.push({
          target,
          suggest: "keep a single UI widget",
          reason: `${widgets.join(" and ")} on one field. Type is not the widget. ${UNUSUAL}`,
        });
      }
      if (ty !== "relationship") {
        if (cards.length > 0 || deps.length > 0 || hasAttr(field, "model")) {
          out.push({
            target,
            suggest: "review relationship-only flags",
            reason: `Default cardinality, dependency, or model(...) usually sits on relationship, not ${ty || "this type"}. ${UNUSUAL}`,
          });
        }
      } else if (!hasAttr(field, "model") && !modelNamed(catalog, field.name)) {
        out.push({
          target,
          suggest: "add model(...)",
          reason: `relationship without model(...) has no target in the default vocabulary. ${UNUSUAL}`,
        });
      }
      if (ty === "relationship" && (hasAttr(field, "Checkbox") || hasAttr(field, "Text"))) {
        out.push({
          target,
          suggest: "use Select or AsyncSelect",
          reason: `Checkbox/Text on relationship is unusual in the default vocabulary. ${UNUSUAL}`,
        });
      }
      if ((BOOLEAN_TYPES.has(ty) || ty === "relationship") && hasAttr(field, "Range")) {
        out.push({
          target,
          suggest: "drop Range",
          reason: `Range on ${ty} is unusual in the default vocabulary. ${UNUSUAL}`,
        });
      }
      if (hasAttr(field, "unsigned") && !INTEGER_TYPES.has(ty)) {
        out.push({
          target,
          suggest: "unsigned on integer widths",
          reason: `unsigned usually pairs with i8/i16/i32/i64, not ${ty || "this type"}. ${UNUSUAL}`,
        });
      }
    }
  }
  return dedupe(out);
}

function syncRisks(catalog: Catalog, keywords: KeywordRow[]): Suggestion[] {
  const out: Suggestion[] = [];
  const byName = new Map(keywords.map((row) => [row.keyword, row]));
  for (const row of keywords) {
    if (row.platforms.length !== 1) {
      continue;
    } else if (SYNC_SENSITIVE.has(row.keyword) || IDENTITY_FIELDS.has(row.keyword.toLowerCase())) {
      out.push({
        target: row.keyword,
        suggest: "document future platforms",
        reason: `${row.keyword} is only listed for ${row.platforms[0]}. Keep it on the contract; add likely future platforms on the keywords.md row. Do not add an ngin target until a second consumer exists.`,
      });
    }
  }
  for (const model of catalog.models) {
    if (hasTrait(model, "Data") || hasTrait(model, "Enum")) {
      const traitName = hasTrait(model, "Data") ? "Data" : "Enum";
      const row = byName.get(traitName);
      if (row && row.platforms.length === 1 && !out.some((item) => item.target === traitName)) {
        out.push({
          target: traitName,
          suggest: "document future platforms",
          reason: `${traitName} is only listed for ${row.platforms[0]}. Tag it once on the contract so a later microservice can share the same shape.`,
        });
      }
    }
    for (const field of model.fields) {
      if (field.type === "relationship" || hasAttr(field, "unique") || hasAttr(field, "email") || hasAttr(field, "phone_no")) {
        const row = byName.get(field.name);
        if (row && row.platforms.length === 1) {
          out.push({
            target: `${model.name}.${field.name}`,
            suggest: "document future platforms",
            reason: `${field.name} is consistency-sensitive and only listed for ${row.platforms[0]}. Keep the attribute on the contract; wait to generate a second platform.`,
          });
        }
      }
    }
  }
  return dedupe(out);
}

function standardRenames(catalog: Catalog): Suggestion[] {
  const names = new Set<string>([
    ...(catalog.types ?? []),
    ...(catalog.traits ?? []),
    ...(catalog.attributes ?? []),
    ...(catalog.fields ?? []),
  ]);
  for (const model of catalog.models) {
    names.add(model.name);
    for (const trait of model.traits) {
      names.add(trait);
    }
    for (const field of model.fields) {
      names.add(field.name);
      if (field.type) {
        names.add(field.type);
      }
      for (const attr of field.attributes) {
        names.add(attr.name);
      }
    }
  }
  const out: Suggestion[] = [];
  for (const name of names) {
    const standard = TYPE_ALIASES[name];
    if (standard) {
      out.push({
        target: name,
        suggest: standard,
        reason: `${name} is an alias; prefer ${standard} on new fields. Extend py_type/ts_type if you adopt it. Do not rewrite existing files unless asked.`,
      });
    }
  }
  return out;
}

function hasTrait(model: CatalogModel, name: string): boolean {
  return model.traits.includes(name);
}

function hasAttr(field: CatalogField, name: string): boolean {
  return field.attributes.some((attr) => attr.name === name);
}

function attrsIn(field: CatalogField, names: Set<string>): string[] {
  return field.attributes.map((attr) => attr.name).filter((name) => names.has(name));
}

function modelNamed(catalog: Catalog, name: string): string | undefined {
  const lower = name.toLowerCase();
  for (const model of catalog.models) {
    if (model.name.toLowerCase() === lower) {
      return model.name;
    }
  }
}

function dedupe(items: Suggestion[]): Suggestion[] {
  const seen = new Set<string>();
  const out: Suggestion[] = [];
  for (const item of items) {
    const key = `${item.target}|${item.suggest}`;
    if (!seen.has(key)) {
      seen.add(key);
      out.push(item);
    }
  }
  return out;
}

function tableCells(line: string): string[] | null {
  const trimmed = line.trim();
  if (!trimmed.startsWith("|")) {
    return null;
  } else {
    const inner = trimmed.replace(/^\|/, "").replace(/\|$/, "");
    return inner.split("|").map((cell) => cell.trim());
  }
}

function isKeywordsHeader(cells: string[]): boolean {
  if (cells.length !== 5) {
    return false;
  } else {
    const keyword = cells[0].toLowerCase();
    return (
      (keyword === "keyword" || keyword === "name") &&
      cells[1].toLowerCase() === "kind" &&
      cells[2].toLowerCase() === "description" &&
      cells[3].toLowerCase() === "purpose" &&
      cells[4].toLowerCase() === "platforms"
    );
  }
}

function isTableSep(cells: string[]): boolean {
  return (
    cells.length > 0 &&
    cells.every((cell) => {
      const stripped = cell.replaceAll(":", "").replaceAll("-", "");
      return cell.length > 0 && stripped.length === 0;
    })
  );
}
