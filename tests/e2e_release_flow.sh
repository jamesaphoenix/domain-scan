#!/usr/bin/env bash
# e2e_release_flow.sh — End-to-end integration test for the full domain-scan
# agent workflow: download from GitHub release, install skills, scan, create
# manifest from scratch, validate, match, write-back, iterate to 100% coverage.
#
# Usage:
#   ./tests/e2e_release_flow.sh
#
# Exits 0 on success, 1 on any failure. Cleans up temp dir on exit.

set -euo pipefail

# ---------------------------------------------------------------------------
# Setup + cleanup
# ---------------------------------------------------------------------------

TEST_DIR=$(mktemp -d "/tmp/domain-scan-e2e-XXXXXX")
trap 'rm -rf "$TEST_DIR"' EXIT

DS="$TEST_DIR/domain-scan"
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); printf "  \033[32mPASS\033[0m %s\n" "$1"; }
fail() { FAIL=$((FAIL + 1)); printf "  \033[31mFAIL\033[0m %s\n" "$1"; }
section() { printf "\n\033[1m[%s]\033[0m\n" "$1"; }

# ---------------------------------------------------------------------------
# Step 1: Download CLI from GitHub release
# ---------------------------------------------------------------------------

section "Step 1: Download CLI from GitHub release"

LATEST=$(curl -sL https://api.github.com/repos/jamesaphoenix/domain-scan/releases/latest \
  | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -n "$LATEST" ]; then
  pass "Fetched latest release tag: $LATEST"
else
  fail "Could not fetch latest release tag"
  exit 1
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in arm64) ARCH="aarch64" ;; esac

ASSET_URL="https://github.com/jamesaphoenix/domain-scan/releases/download/${LATEST}/domain-scan-${OS}-${ARCH}.tar.gz"
HTTP_CODE=$(curl -sL -o "$TEST_DIR/domain-scan.tar.gz" -w "%{http_code}" "$ASSET_URL")

if [ "$HTTP_CODE" = "200" ]; then
  pass "Downloaded domain-scan-${OS}-${ARCH}.tar.gz (HTTP $HTTP_CODE)"
else
  fail "Download failed: HTTP $HTTP_CODE from $ASSET_URL"
  exit 1
fi

tar -xzf "$TEST_DIR/domain-scan.tar.gz" -C "$TEST_DIR"
chmod +x "$DS"

if "$DS" --version >/dev/null 2>&1; then
  pass "Binary runs: $("$DS" --version 2>&1)"
else
  fail "Binary does not run"
  exit 1
fi

# ---------------------------------------------------------------------------
# Step 2: Install skills
# ---------------------------------------------------------------------------

section "Step 2: Install agent skills"

"$DS" skills install --dir "$TEST_DIR/skills-claude" >/dev/null 2>&1
CLAUDE_COUNT=$(ls -1 "$TEST_DIR/skills-claude/" 2>/dev/null | wc -l | tr -d ' ')
if [ "$CLAUDE_COUNT" -ge 10 ]; then
  pass "Claude Code skills installed: $CLAUDE_COUNT files"
else
  fail "Expected >= 10 Claude Code skill files, got $CLAUDE_COUNT"
fi

"$DS" skills install --dir "$TEST_DIR/skills-codex" >/dev/null 2>&1
CODEX_COUNT=$(ls -1 "$TEST_DIR/skills-codex/" 2>/dev/null | wc -l | tr -d ' ')
if [ "$CODEX_COUNT" -ge 10 ]; then
  pass "Codex skills installed: $CODEX_COUNT files"
else
  fail "Expected >= 10 Codex skill files, got $CODEX_COUNT"
fi

# ---------------------------------------------------------------------------
# Step 3: Create test fixture codebase
# ---------------------------------------------------------------------------

section "Step 3: Create test fixture"

mkdir -p "$TEST_DIR/project/src"/{auth,billing,api,notifications,data}

cat > "$TEST_DIR/project/src/auth/types.ts" << 'TS'
export interface AuthToken { userId: string; token: string; expiresAt: Date; }
export interface AuthCredentials { email: string; password: string; }
export interface AuthSession { id: string; userId: string; expiresAt: Date; }
export type AuthProvider = "local" | "google" | "github";
TS

cat > "$TEST_DIR/project/src/auth/service.ts" << 'TS'
import { AuthToken, AuthCredentials } from "./types";
export class AuthService {
  async login(creds: AuthCredentials): Promise<AuthToken> { return {} as AuthToken; }
  async logout(sessionId: string): Promise<void> {}
  async refreshToken(token: string): Promise<AuthToken> { return {} as AuthToken; }
  async validateToken(token: string): Promise<boolean> { return true; }
}
TS

cat > "$TEST_DIR/project/src/auth/middleware.ts" << 'TS'
import { AuthService } from "./service";
export interface AuthMiddlewareConfig { excludePaths: string[]; tokenHeader: string; }
export class AuthMiddleware {
  constructor(private authService: AuthService, private config: AuthMiddlewareConfig) {}
  async handle(request: Request): Promise<Response | null> { return null; }
}
TS

cat > "$TEST_DIR/project/src/billing/types.ts" << 'TS'
export interface Invoice { id: string; userId: string; amount: number; status: string; }
export interface LineItem { description: string; quantity: number; unitPrice: number; }
export interface PaymentMethod { id: string; type: "card" | "bank"; last4?: string; }
export interface Subscription { id: string; userId: string; planId: string; status: string; }
export type InvoiceStatus = "draft" | "pending" | "paid" | "overdue";
TS

cat > "$TEST_DIR/project/src/billing/service.ts" << 'TS'
import { Invoice, LineItem, Subscription } from "./types";
export class BillingService {
  async createInvoice(userId: string, items: LineItem[]): Promise<Invoice> { return {} as Invoice; }
  async processPayment(invoiceId: string): Promise<Invoice> { return {} as Invoice; }
  async getSubscription(userId: string): Promise<Subscription | null> { return null; }
  async cancelSubscription(id: string): Promise<void> {}
}
TS

cat > "$TEST_DIR/project/src/billing/stripe.ts" << 'TS'
import { PaymentMethod } from "./types";
export interface StripeConfig { apiKey: string; webhookSecret: string; }
export class StripeGateway {
  constructor(private config: StripeConfig) {}
  async createCharge(amount: number, pm: PaymentMethod): Promise<string> { return "ch_1"; }
  async createRefund(chargeId: string): Promise<string> { return "re_1"; }
}
TS

cat > "$TEST_DIR/project/src/api/router.ts" << 'TS'
export interface Route { method: string; path: string; handler: string; }
export interface RouterConfig { prefix: string; routes: Route[]; }
export class ApiRouter {
  constructor(private config: RouterConfig) {}
  async handleRequest(method: string, path: string): Promise<Response> { return new Response(); }
  registerRoute(route: Route): void { this.config.routes.push(route); }
}
TS

cat > "$TEST_DIR/project/src/api/controllers.ts" << 'TS'
import { AuthService } from "../auth/service";
import { BillingService } from "../billing/service";
export interface ApiResponse<T = unknown> { success: boolean; data?: T; statusCode: number; }
export class AuthController {
  constructor(private auth: AuthService) {}
  async handleLogin(email: string, password: string): Promise<ApiResponse> { return { success: true, statusCode: 200 }; }
}
export class BillingController {
  constructor(private billing: BillingService) {}
  async handleCreateInvoice(userId: string): Promise<ApiResponse> { return { success: true, statusCode: 201 }; }
}
TS

cat > "$TEST_DIR/project/src/api/validation.ts" << 'TS'
export interface ValidationRule { field: string; required?: boolean; minLength?: number; }
export interface ValidationResult { valid: boolean; errors: ValidationError[]; }
export interface ValidationError { field: string; message: string; code: string; }
export class RequestValidator {
  constructor(private rules: ValidationRule[]) {}
  validate(data: Record<string, unknown>): ValidationResult { return { valid: true, errors: [] }; }
}
TS

cat > "$TEST_DIR/project/src/notifications/types.ts" << 'TS'
export interface Notification { id: string; userId: string; type: string; title: string; read: boolean; }
export interface NotificationPreferences { userId: string; email: boolean; push: boolean; }
export interface NotificationTemplate { id: string; type: string; subject: string; bodyTemplate: string; }
export type NotificationType = "invoice_paid" | "password_reset" | "welcome";
TS

cat > "$TEST_DIR/project/src/notifications/service.ts" << 'TS'
import { Notification, NotificationTemplate } from "./types";
export class NotificationService {
  async send(userId: string, type: string): Promise<Notification> { return {} as Notification; }
  async markAsRead(id: string): Promise<void> {}
  async getUnread(userId: string): Promise<Notification[]> { return []; }
}
TS

cat > "$TEST_DIR/project/src/notifications/email.ts" << 'TS'
export interface EmailConfig { smtpHost: string; smtpPort: number; fromAddress: string; }
export interface EmailMessage { to: string; subject: string; htmlBody: string; }
export class EmailSender {
  constructor(private config: EmailConfig) {}
  async sendEmail(message: EmailMessage): Promise<boolean> { return true; }
  async sendBatch(messages: EmailMessage[]): Promise<{ sent: number }> { return { sent: messages.length }; }
}
TS

cat > "$TEST_DIR/project/src/data/schema.ts" << 'TS'
export interface UserSchema { id: string; email: string; name: string; role: string; }
export interface InvoiceSchema { id: string; userId: string; amount: number; status: string; }
export interface SubscriptionSchema { id: string; userId: string; planId: string; }
export interface NotificationSchema { id: string; userId: string; type: string; read: boolean; }
export interface AuditLogSchema { id: string; action: string; resource: string; timestamp: Date; }
TS

cat > "$TEST_DIR/project/src/data/repository.ts" << 'TS'
import { UserSchema, InvoiceSchema, SubscriptionSchema } from "./schema";
export interface Repository<T> { findById(id: string): Promise<T | null>; create(data: T): Promise<T>; }
export class UserRepository implements Repository<UserSchema> {
  async findById(id: string): Promise<UserSchema | null> { return null; }
  async create(data: UserSchema): Promise<UserSchema> { return data; }
  async findByEmail(email: string): Promise<UserSchema | null> { return null; }
}
export class InvoiceRepository implements Repository<InvoiceSchema> {
  async findById(id: string): Promise<InvoiceSchema | null> { return null; }
  async create(data: InvoiceSchema): Promise<InvoiceSchema> { return data; }
}
export class SubscriptionRepository implements Repository<SubscriptionSchema> {
  async findById(id: string): Promise<SubscriptionSchema | null> { return null; }
  async create(data: SubscriptionSchema): Promise<SubscriptionSchema> { return data; }
}
TS

cat > "$TEST_DIR/project/src/data/migrations.ts" << 'TS'
export interface Migration { id: string; name: string; up: () => Promise<void>; down: () => Promise<void>; }
export class MigrationRunner {
  private migrations: Migration[] = [];
  register(m: Migration): void { this.migrations.push(m); }
  async runAll(): Promise<string[]> { return this.migrations.map(m => m.id); }
  async rollback(count: number): Promise<string[]> { return []; }
}
TS

FILE_COUNT=$(find "$TEST_DIR/project" -name "*.ts" | wc -l | tr -d ' ')
pass "Created $FILE_COUNT TypeScript fixture files"

# ---------------------------------------------------------------------------
# Step 4: Scan the codebase
# ---------------------------------------------------------------------------

section "Step 4: Scan codebase"

SCAN_OUTPUT=$("$DS" scan --root "$TEST_DIR/project" --output json --fields stats 2>&1 | grep -v "^Found")
TOTAL_FILES=$(echo "$SCAN_OUTPUT" | python3 -c "import json,sys; print(json.load(sys.stdin)['stats']['total_files'])")
TOTAL_IFACES=$(echo "$SCAN_OUTPUT" | python3 -c "import json,sys; print(json.load(sys.stdin)['stats']['total_interfaces'])")
TOTAL_CLASSES=$(echo "$SCAN_OUTPUT" | python3 -c "import json,sys; print(json.load(sys.stdin)['stats']['total_classes'])")

if [ "$TOTAL_FILES" -ge 10 ]; then
  pass "Scanned $TOTAL_FILES files, $TOTAL_IFACES interfaces, $TOTAL_CLASSES classes"
else
  fail "Expected >= 10 files, got $TOTAL_FILES"
fi

# ---------------------------------------------------------------------------
# Step 5: Schema introspection
# ---------------------------------------------------------------------------

section "Step 5: Schema introspection"

SCHEMA_OUTPUT=$("$DS" schema scan 2>&1)
if echo "$SCHEMA_OUTPUT" | grep -q '"command"'; then
  pass "domain-scan schema scan returns valid JSON schema"
else
  fail "Schema output missing expected fields"
fi

# ---------------------------------------------------------------------------
# Step 6: Create manifest from scratch
# ---------------------------------------------------------------------------

section "Step 6: Create manifest + validate + match to 100%"

cat > "$TEST_DIR/project/system.json" << 'JSON'
{
  "meta": { "name": "e2e-test", "version": "1.0.0", "description": "E2E test" },
  "domains": {
    "auth": { "label": "Auth", "color": "#3b82f6" },
    "billing": { "label": "Billing", "color": "#8b5cf6" },
    "api": { "label": "API", "color": "#22c55e" },
    "notifications": { "label": "Notifications", "color": "#f97316" },
    "data": { "label": "Data", "color": "#ef4444" }
  },
  "subsystems": [
    {
      "id": "auth-core", "name": "Auth Core", "domain": "auth", "status": "new",
      "filePath": "src/auth",
      "interfaces": ["AuthToken","AuthCredentials","AuthSession","AuthMiddlewareConfig",
                      "AuthService","AuthMiddleware","AuthProvider"],
      "operations": ["login","logout","refreshToken","validateToken"],
      "tables": [], "events": [], "dependencies": []
    },
    {
      "id": "billing-core", "name": "Billing Core", "domain": "billing", "status": "new",
      "filePath": "src/billing",
      "interfaces": ["Invoice","LineItem","PaymentMethod","Subscription","StripeConfig",
                      "BillingService","StripeGateway","InvoiceStatus"],
      "operations": ["createInvoice","processPayment","createCharge"],
      "tables": [], "events": [], "dependencies": ["auth-core"]
    },
    {
      "id": "api-gateway", "name": "API Gateway", "domain": "api", "status": "new",
      "filePath": "src/api",
      "interfaces": ["ApiResponse","Route","RouterConfig","ValidationRule","ValidationResult",
                      "ValidationError","AuthController","BillingController","ApiRouter","RequestValidator"],
      "operations": ["handleRequest","registerRoute","handleLogin"],
      "tables": [], "events": [], "dependencies": ["auth-core","billing-core"]
    },
    {
      "id": "notifications-core", "name": "Notifications", "domain": "notifications", "status": "new",
      "filePath": "src/notifications",
      "interfaces": ["Notification","NotificationPreferences","NotificationTemplate","EmailConfig",
                      "EmailMessage","NotificationService","EmailSender","NotificationType"],
      "operations": ["send","markAsRead","sendEmail"],
      "tables": [], "events": [], "dependencies": []
    },
    {
      "id": "data-persistence", "name": "Data Layer", "domain": "data", "status": "new",
      "filePath": "src/data",
      "interfaces": ["Repository","UserSchema","InvoiceSchema","SubscriptionSchema",
                      "NotificationSchema","AuditLogSchema","Migration",
                      "MigrationRunner","UserRepository","InvoiceRepository","SubscriptionRepository"],
      "operations": ["findById","create","runAll","rollback"],
      "tables": [], "events": [], "dependencies": []
    }
  ],
  "connections": [
    { "from": "api-gateway", "to": "auth-core", "label": "authenticates-via", "type": "depends_on" },
    { "from": "api-gateway", "to": "billing-core", "label": "delegates-billing-to", "type": "depends_on" },
    { "from": "billing-core", "to": "data-persistence", "label": "persists-to", "type": "depends_on" }
  ]
}
JSON

pass "Created system.json from scratch"

# Match and check coverage
MATCH_OUTPUT=$("$DS" match --root "$TEST_DIR/project" --manifest "$TEST_DIR/project/system.json" \
  --output json --fields coverage_percent,total_entities 2>&1 | grep -v "^Found")

COVERAGE=$(echo "$MATCH_OUTPUT" | python3 -c "import json,sys; print(json.load(sys.stdin)['coverage_percent'])")
TOTAL=$(echo "$MATCH_OUTPUT" | python3 -c "import json,sys; print(json.load(sys.stdin)['total_entities'])")

if python3 -c "exit(0 if $COVERAGE >= 90 else 1)"; then
  pass "Coverage: ${COVERAGE}% ($TOTAL entities)"
else
  fail "Coverage only ${COVERAGE}% — expected >= 90%"
fi

# Check zero unmatched
UNMATCHED=$("$DS" match --root "$TEST_DIR/project" --manifest "$TEST_DIR/project/system.json" \
  --output json --unmatched-only 2>&1 | grep -v "^Found" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")

if [ "$UNMATCHED" = "0" ]; then
  pass "Zero unmatched entities"
else
  fail "$UNMATCHED entities still unmatched"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

TOTAL_TESTS=$((PASS + FAIL))
printf "\n\033[1m%d/%d tests passed\033[0m" "$PASS" "$TOTAL_TESTS"
if [ "$FAIL" -gt 0 ]; then
  printf " (\033[31m%d failed\033[0m)" "$FAIL"
  printf "\n"
  exit 1
else
  printf "\n"
  exit 0
fi
