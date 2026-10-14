#!/usr/bin/env node
// PreToolUse gate on ExitPlanMode. Enforces the CLAUDE.md working agreement: a plan
// may not be submitted for approval unless it states (1) explicit success criteria and
// (2) a named precedent/example to match. On a miss, denies the tool call with a reason
// that names what's missing, so the plan gets revised and resubmitted. Fails open
// (allow) on any parse/read problem so it never wedges planning.

let raw = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  raw += chunk;
});
process.stdin.on("end", () => {
  let plan = "";
  try {
    plan = String(JSON.parse(raw)?.tool_input?.plan ?? "");
  } catch {
    process.exit(0); // unparseable input — don't block
  }
  if (!plan.trim()) process.exit(0);

  const text = plan.toLowerCase();
  const missing = [];

  // (1) Explicit success criteria / definition of done.
  if (!/(success criteria|definition of done|acceptance criteria)/.test(text)) {
    missing.push('an explicit "Success criteria" (definition-of-done) section');
  }

  // (2) A named precedent/example to match — or an explicit "no precedent" note.
  if (
    !/(precedent|example to match|match the|to match\b|mirror|follows the|pattern of|no precedent)/.test(
      text,
    )
  ) {
    missing.push(
      'a named precedent/example to match (or an explicit "no precedent exists" note + the format to follow)',
    );
  }

  if (missing.length === 0) process.exit(0);

  const reason =
    "Plan blocked by the working agreement (CLAUDE.md § “Working agreement — every plan & change”). " +
    "Add " +
    missing.join(" and ") +
    ", then resubmit.";

  process.stdout.write(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        permissionDecision: "deny",
        permissionDecisionReason: reason,
      },
    }),
  );
  process.exit(0);
});
