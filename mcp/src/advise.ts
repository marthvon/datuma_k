export type AdviseResult = {
  use_ngin: boolean;
  reasons: string[];
};

const GLUE =
  /\b(rout(e|ing)|auth(entication|z)?|oauth|middleware|styling|css|tailwind|business rule|payment|handwritten|one-off|one off)\b/i;
const CONTRACT =
  /\b(field|type|validat|form|schema|model|contract|crud|dto|pydantic|zod|serializer|attribute|trait)\b/i;

export function adviseNgin(task: string, platforms: string[]): AdviseResult {
  const reasons: string[] = [];
  const unique = [...new Set(platforms.filter((name) => name.length > 0))];
  const glue = GLUE.test(task);
  const derived = CONTRACT.test(task);
  const multi = unique.length >= 2;
  if (multi && derived) {
    reasons.push(
      "Multiple platforms need the same contract-derived types, validation, or UI — put that in ngin.",
    );
  } else if (!multi) {
    reasons.push("ngin pays off when two or more platforms must stay in sync on the same shape.");
  } else {
    reasons.push("Task does not look derived from dtct fields, traits, or attributes.");
  }
  if (glue) {
    reasons.push(
      "Routing, auth, HTTP glue, styling, and business rules stay handwritten; do not emit them from ngin.",
    );
  }
  if (multi && derived) {
    reasons.push("Edits inside generated spans are overwritten on the next datuma_k run; keep handwritten code between spans.");
  }
  return { use_ngin: multi && derived, reasons };
}
