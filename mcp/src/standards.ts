export const STANDARD_TYPES = [
  "i8",
  "i16",
  "i32",
  "i64",
  "float",
  "double",
  "string",
  "boolean",
  "datetime",
  "relationship",
] as const;

export const STANDARD_TRAITS = ["Data", "Enum"] as const;

export const CARDINALITY_ATTRS = new Set(["OneToOne", "OneToMany", "ManyToMany", "BelongsTo"]);

export const DEPENDENCY_ATTRS = new Set(["Partial", "Full", "Transitive"]);

export const UI_WIDGETS = new Set([
  "Text",
  "Select",
  "Checkbox",
  "Range",
  "DateRange",
  "Date",
  "Datetime",
  "DatetimeRange",
  "Radio",
  "AsyncSelect",
  "Upload",
]);

export const STANDARD_ATTRIBUTES = [
  "nullable",
  "unique",
  "default",
  "email",
  "phone_no",
  "unsigned",
  "required",
  "min",
  "max",
  "min_length",
  "max_length",
  "regex",
  "local",
  "model",
  ...CARDINALITY_ATTRS,
  ...DEPENDENCY_ATTRS,
  ...UI_WIDGETS,
] as const;

export const TYPE_ALIASES: Record<string, string> = {
  text_type: "string",
  int_type: "i32",
  bool_type: "boolean",
  datetime_type: "datetime",
};

export const INTEGER_TYPES = new Set<string>(["i8", "i16", "i32", "i64"]);

export const FLOAT_TYPES = new Set<string>(["float", "double"]);

export const NUMERIC_TYPES = new Set<string>([...INTEGER_TYPES, ...FLOAT_TYPES]);

export const STRING_TYPES = new Set<string>(["string", "text_type"]);

export const BOOLEAN_TYPES = new Set<string>(["boolean", "bool_type"]);

export const DATETIME_TYPES = new Set<string>(["datetime", "datetime_type"]);

export const SYNC_SENSITIVE = new Set<string>([
  "Data",
  "Enum",
  "unique",
  "relationship",
  "email",
  "phone_no",
  "model",
  ...CARDINALITY_ATTRS,
  ...DEPENDENCY_ATTRS,
]);

export const IDENTITY_FIELDS = new Set<string>(["id", "uuid", "sku", "code"]);

export const NGIN_HELPER_MAP: Record<string, { python: string; typescript: string; zod: string }> = {
  i8: { python: "int", typescript: "number", zod: "z.number().int()" },
  i16: { python: "int", typescript: "number", zod: "z.number().int()" },
  i32: { python: "int", typescript: "number", zod: "z.number().int()" },
  i64: { python: "int", typescript: "number", zod: "z.number().int()" },
  float: { python: "float", typescript: "number", zod: "z.number()" },
  double: { python: "float", typescript: "number", zod: "z.number()" },
  string: { python: "str", typescript: "string", zod: "z.string()" },
  boolean: { python: "bool", typescript: "boolean", zod: "z.boolean()" },
  datetime: { python: "datetime", typescript: "Date", zod: "z.string()" },
  relationship: { python: "int", typescript: "number", zod: "z.number().int()" },
};
