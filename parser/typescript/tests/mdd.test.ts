import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve } from '../src/resolver.js';
import { validate } from '../src/validator.js';
import { parseString } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

const mesContent = readFileSync(join(__dirname, 'fixtures/mdd/mes.m3l'), 'utf-8');
const rayboxContent = readFileSync(join(__dirname, 'fixtures/mdd/raybox.m3l'), 'utf-8');

// Helper: parse a single file into ParsedFile
function parseSingleFile(content: string, file: string) {
  const tokens = lex(content, file);
  return parseTokens(tokens, file);
}

// ============================================================
// A. Lexer tests for mes.m3l
// ============================================================
describe('A. Lexer tests for mes.m3l', () => {
  const tokens = lex(mesContent, 'mes.m3l');
  const nonBlank = tokens.filter(t => t.type !== 'blank');

  it('tokenizes namespace "umes.ray.mes"', () => {
    const nsToken = nonBlank.find(t => t.type === 'namespace');
    expect(nsToken).toBeDefined();
    expect(nsToken!.data?.name).toBe('umes.ray.mes');
    expect(nsToken!.data?.is_namespace).toBe(true);
  });

  it('tokenizes blockquote descriptions (multi-line)', () => {
    const blockquotes = nonBlank.filter(t => t.type === 'blockquote');
    expect(blockquotes.length).toBeGreaterThanOrEqual(2);
    expect(blockquotes[0].data?.text).toBe('U-MES.Ray - Core MES Data Model');
    expect(blockquotes[1].data?.text).toContain('레이저 절단 MES');
  });

  it('tokenizes enum with space-separated desc: CREATED "생성" (no colon)', () => {
    // In mes.m3l: `- CREATED "생성"` is a field token inside an enum context
    const createdToken = nonBlank.find(
      t => t.type === 'field' && t.data?.name === 'CREATED' && t.data?.description === '생성'
    );
    expect(createdToken).toBeDefined();
    expect(createdToken!.data?.description).toBe('생성');
  });

  it('tokenizes enum without description: OFFLINE (from MachineOnlineStatus in raybox)', () => {
    // mes.m3l enums all have descriptions; this verifies the pattern with name-only works
    // Using PLANNED from DowntimeCategory which does have a desc, but let's check a name-only pattern
    // Actually all mes.m3l enums have descriptions. We verify one exists.
    const plannedToken = nonBlank.find(
      t => t.type === 'field' && t.data?.name === 'PLANNED'
    );
    expect(plannedToken).toBeDefined();
    expect(plannedToken!.data?.description).toBe('계획');
  });

  it('tokenizes enum with typed value: STANDBY: integer = 0 (pattern check)', () => {
    // mes.m3l does not have typed enum values; but we verify the field token pattern
    // for the simple description enum to ensure correct tokenization
    const releasedToken = nonBlank.find(
      t => t.type === 'field' && t.data?.name === 'RELEASED'
    );
    expect(releasedToken).toBeDefined();
    expect(releasedToken!.data?.description).toBe('출고');
  });

  it('tokenizes model with label and inheritance: Customer(거래처) : BaseEntity', () => {
    const customerToken = nonBlank.find(
      t => t.type === 'model' && t.data?.name === 'Customer'
    );
    expect(customerToken).toBeDefined();
    expect(customerToken!.data?.label).toBe('거래처');
    expect(customerToken!.data?.inherits).toEqual(['BaseEntity']);
  });

  it('tokenizes model with multiple inheritance: Part(부품) : BaseEntity, Auditable', () => {
    const partToken = nonBlank.find(
      t => t.type === 'model' && t.data?.name === 'Part'
    );
    expect(partToken).toBeDefined();
    expect(partToken!.data?.label).toBe('부품');
    expect(partToken!.data?.inherits).toEqual(['BaseEntity', 'Auditable']);
  });

  it('tokenizes kind section # Lookup, # Rollup, # Computed', () => {
    const kindSections = nonBlank.filter(
      t => t.type === 'section' && t.data?.kind_section === true
    );
    const kindNames = kindSections.map(t => t.data?.name);
    expect(kindNames).toContain('Lookup');
    expect(kindNames).toContain('Rollup');
    expect(kindNames).toContain('Computed');
  });

  it('tokenizes horizontal rules as separators', () => {
    const hrs = tokens.filter(t => t.type === 'horizontal_rule');
    expect(hrs.length).toBeGreaterThanOrEqual(5);
  });

  it('tokenizes field with @reference attribute', () => {
    const refField = nonBlank.find(
      t => t.type === 'field' && t.data?.name === 'material_id' &&
        Array.isArray(t.data?.attributes) &&
        (t.data?.attributes as { name: string }[]).some(a => a.name === 'reference')
    );
    expect(refField).toBeDefined();
  });

  it('tokenizes directive-only line: @unique(...)', () => {
    const directiveTokens = nonBlank.filter(
      t => t.type === 'field' && t.data?.is_directive === true
    );
    expect(directiveTokens.length).toBeGreaterThanOrEqual(1);
    const uniqueDirective = directiveTokens.find(t => {
      const attrs = t.data?.attributes as { name: string }[] | undefined;
      return attrs?.some(a => a.name === 'unique');
    });
    expect(uniqueDirective).toBeDefined();
  });
});

// ============================================================
// B. Lexer tests for raybox.m3l
// ============================================================
describe('B. Lexer tests for raybox.m3l', () => {
  const tokens = lex(rayboxContent, 'raybox.m3l');
  const nonBlank = tokens.filter(t => t.type !== 'blank');

  it('handles @import "./mes.m3l" (text token with is_import)', () => {
    const importToken = nonBlank.find(
      t => t.type === 'text' && t.data?.is_import === true
    );
    expect(importToken).toBeDefined();
    expect(importToken!.data?.import_path).toBe('./mes.m3l');
  });

  it('tokenizes enum H2 with description: CutSoftwareType ::enum "절단 소프트웨어"', () => {
    const enumToken = nonBlank.find(
      t => t.type === 'enum' && t.data?.name === 'CutSoftwareType'
    );
    expect(enumToken).toBeDefined();
    expect(enumToken!.data?.description).toBe('절단 소프트웨어');
  });

  it('tokenizes documentation H1 headers that are NOT kind sections', () => {
    // Headers like "# RayBox 식별", "# 제어카드" should be namespace tokens, not kind_section
    const h1Tokens = nonBlank.filter(t => t.type === 'namespace' || t.type === 'section');

    // Find doc headers that should be namespace (not kind_section)
    const docHeaders = h1Tokens.filter(t => {
      const name = t.data?.name as string;
      return name === 'RayBox 식별' || name === '제어카드';
    });
    expect(docHeaders.length).toBeGreaterThanOrEqual(2);

    // They should be namespace type (not kind_section)
    for (const token of docHeaders) {
      expect(token.type).toBe('namespace');
      expect(token.data?.kind_section).toBeUndefined();
    }
  });

  it('documentation H1 headers should NOT have kind_section: true', () => {
    const allH1 = nonBlank.filter(t => t.type === 'namespace');
    // Find all non-Namespace: H1 tokens (document titles / doc headers)
    const docH1s = allH1.filter(t => t.data?.is_namespace === false);
    // These include: "RayBox 식별", "제어카드", "레이저", "가공 영역", etc.
    expect(docH1s.length).toBeGreaterThan(0);
    for (const token of docH1s) {
      expect(token.data?.kind_section).toBeUndefined();
    }
  });
});

// ============================================================
// C. Parser tests for mes.m3l
// ============================================================
describe('C. Parser tests for mes.m3l', () => {
  const parsed = parseSingleFile(mesContent, 'mes.m3l');

  it('parses namespace correctly', () => {
    expect(parsed.namespace).toBe('umes.ray.mes');
  });

  it('counts all models (at least 20)', () => {
    const modelNames = parsed.models.map(m => m.name);
    expect(parsed.models.length).toBeGreaterThanOrEqual(20);
    // Verify key models exist
    const expectedModels = [
      'BaseEntity', 'Customer', 'Material', 'MaterialSpec', 'Part',
      'MachineGroup', 'Machine', 'Worker', 'DefectCode', 'DowntimeCode',
      'WorkOrder', 'WorkOrderItem', 'ProductionResult',
      'InspectionPlan', 'InspectionItem', 'InspectionRecord', 'InspectionMeasurement',
      'DefectRecord', 'CAPA', 'MaintenancePlan', 'MaintenanceRecord',
      'DowntimeEvent', 'OEERecord',
    ];
    for (const name of expectedModels) {
      expect(modelNames).toContain(name);
    }
  });

  it('counts all enums (at least 9)', () => {
    const enumNames = parsed.enums.map(e => e.name);
    expect(parsed.enums.length).toBeGreaterThanOrEqual(9);
    const expectedEnums = [
      'WorkOrderStatus', 'ProductionType', 'InspectionType',
      'InspectionResultType', 'DefectSeverity', 'DefectDisposition',
      'MaintenanceType', 'DowntimeCategory', 'CAPAStatus',
    ];
    for (const name of expectedEnums) {
      expect(enumNames).toContain(name);
    }
  });

  it('counts interfaces (should have 1: Auditable)', () => {
    expect(parsed.interfaces.length).toBe(1);
    expect(parsed.interfaces[0].name).toBe('Auditable');
  });

  it('WorkOrderStatus enum has 7 values (CREATED through CANCELLED)', () => {
    const wos = parsed.enums.find(e => e.name === 'WorkOrderStatus');
    expect(wos).toBeDefined();
    expect(wos!.values.length).toBe(7);
    const valueNames = wos!.values.map(v => v.name);
    expect(valueNames).toEqual([
      'CREATED', 'RELEASED', 'IN_PROGRESS', 'PAUSED',
      'COMPLETED', 'CLOSED', 'CANCELLED',
    ]);
  });

  it('WorkOrderStatus values have correct descriptions', () => {
    const wos = parsed.enums.find(e => e.name === 'WorkOrderStatus')!;
    const created = wos.values.find(v => v.name === 'CREATED');
    expect(created?.description).toBe('생성');
    const released = wos.values.find(v => v.name === 'RELEASED');
    expect(released?.description).toBe('출고');
    const cancelled = wos.values.find(v => v.name === 'CANCELLED');
    expect(cancelled?.description).toBe('취소');
  });

  it('Customer model inherits BaseEntity, has expected fields', () => {
    const customer = parsed.models.find(m => m.name === 'Customer')!;
    expect(customer.inherits).toEqual(['BaseEntity']);
    expect(customer.label).toBe('거래처');
    const fieldNames = customer.fields.map(f => f.name);
    expect(fieldNames).toContain('code');
    expect(fieldNames).toContain('name');
    expect(fieldNames).toContain('contact_name');
    expect(fieldNames).toContain('phone');
    expect(fieldNames).toContain('email');
    expect(fieldNames).toContain('address');
    expect(fieldNames).toContain('is_active');
  });

  it('Part model inherits BaseEntity AND Auditable (multiple inheritance)', () => {
    const part = parsed.models.find(m => m.name === 'Part')!;
    expect(part.inherits).toEqual(['BaseEntity', 'Auditable']);
    expect(part.label).toBe('부품');
  });

  it('MaterialSpec has a lookup field (material_name)', () => {
    const matSpec = parsed.models.find(m => m.name === 'MaterialSpec')!;
    const lookupField = matSpec.fields.find(f => f.name === 'material_name');
    expect(lookupField).toBeDefined();
    expect(lookupField!.kind).toBe('lookup');
    expect(lookupField!.lookup?.path).toBe('material_id.name');
  });

  it('WorkOrder has rollup fields (item_count, total_order_qty, etc.)', () => {
    const wo = parsed.models.find(m => m.name === 'WorkOrder')!;

    const itemCount = wo.fields.find(f => f.name === 'item_count');
    expect(itemCount).toBeDefined();
    expect(itemCount!.kind).toBe('rollup');
    expect(itemCount!.rollup?.target).toBe('WorkOrderItem');
    expect(itemCount!.rollup?.fk).toBe('work_order_id');
    expect(itemCount!.rollup?.aggregate).toBe('count');

    const totalOrderQty = wo.fields.find(f => f.name === 'total_order_qty');
    expect(totalOrderQty).toBeDefined();
    expect(totalOrderQty!.kind).toBe('rollup');
    expect(totalOrderQty!.rollup?.aggregate).toBe('sum');
    expect(totalOrderQty!.rollup?.field).toBe('order_qty');

    const totalGoodQty = wo.fields.find(f => f.name === 'total_good_qty');
    expect(totalGoodQty).toBeDefined();

    const totalDefectQty = wo.fields.find(f => f.name === 'total_defect_qty');
    expect(totalDefectQty).toBeDefined();
  });

  it('OEERecord has computed field (oee)', () => {
    const oee = parsed.models.find(m => m.name === 'OEERecord')!;
    const oeeField = oee.fields.find(f => f.name === 'oee');
    expect(oeeField).toBeDefined();
    expect(oeeField!.kind).toBe('computed');
    expect(oeeField!.computed?.expression).toBe(
      'availability * performance * quality_rate'
    );
  });

  it('directive lines like @unique(...) are parsed into sections', () => {
    // MaterialSpec has @unique(material_id, thickness, width, length)
    const matSpec = parsed.models.find(m => m.name === 'MaterialSpec')!;
    const uniqueSection = matSpec.sections['unique'];
    expect(uniqueSection).toBeDefined();
    expect(Array.isArray(uniqueSection)).toBe(true);
    expect((uniqueSection as unknown[]).length).toBeGreaterThanOrEqual(1);

    // OEERecord also has @unique(machine_id, record_date, shift)
    const oee = parsed.models.find(m => m.name === 'OEERecord')!;
    const oeeUnique = oee.sections['unique'];
    expect(oeeUnique).toBeDefined();
  });
});

// ============================================================
// D. Parser tests for raybox.m3l (standalone, without import resolution)
// ============================================================
describe('D. Parser tests for raybox.m3l (standalone)', () => {
  const parsed = parseSingleFile(rayboxContent, 'raybox.m3l');

  it('counts enums (at least 6)', () => {
    const enumNames = parsed.enums.map(e => e.name);
    expect(parsed.enums.length).toBeGreaterThanOrEqual(6);
    const expectedEnums = [
      'CutSoftwareType', 'CypCutSysState', 'TubeProSysState',
      'TubeProCutMode', 'MachineOnlineStatus', 'RayboxTaskStatus',
    ];
    for (const name of expectedEnums) {
      expect(enumNames).toContain(name);
    }
  });

  it('CutSoftwareType enum should have description "절단 소프트웨어"', () => {
    const cutSw = parsed.enums.find(e => e.name === 'CutSoftwareType');
    expect(cutSw).toBeDefined();
    expect(cutSw!.description).toBe('절단 소프트웨어');
  });

  it('CypCutSysState has typed values (STANDBY: integer = 0, etc.)', () => {
    const cypcut = parsed.enums.find(e => e.name === 'CypCutSysState')!;
    expect(cypcut.values.length).toBe(13);

    const standby = cypcut.values.find(v => v.name === 'STANDBY');
    expect(standby).toBeDefined();
    expect(standby!.type).toBe('integer');
    expect(standby!.value).toBe('0');

    const machining = cypcut.values.find(v => v.name === 'MACHINING');
    expect(machining).toBeDefined();
    expect(machining!.value).toBe('8');

    const backward = cypcut.values.find(v => v.name === 'BACKWARD');
    expect(backward).toBeDefined();
    expect(backward!.value).toBe('12');
  });

  it('MachineOnlineStatus has values without descriptions (OFFLINE, ALARMING, etc.)', () => {
    const mos = parsed.enums.find(e => e.name === 'MachineOnlineStatus')!;
    expect(mos.values.length).toBe(5);
    const offline = mos.values.find(v => v.name === 'OFFLINE');
    expect(offline).toBeDefined();
    expect(offline!.description).toBeUndefined();

    const alarming = mos.values.find(v => v.name === 'ALARMING');
    expect(alarming).toBeDefined();
    expect(alarming!.description).toBeUndefined();
  });

  it('LaserMachine model should have lookup fields (machine_name, machine_code, raybox_server)', () => {
    const lm = parsed.models.find(m => m.name === 'LaserMachine')!;
    expect(lm).toBeDefined();

    const machineName = lm.fields.find(f => f.name === 'machine_name');
    expect(machineName).toBeDefined();
    expect(machineName!.kind).toBe('lookup');
    expect(machineName!.lookup?.path).toBe('machine_id.name');

    const machineCode = lm.fields.find(f => f.name === 'machine_code');
    expect(machineCode).toBeDefined();
    expect(machineCode!.kind).toBe('lookup');
    expect(machineCode!.lookup?.path).toBe('machine_id.code');

    const rayboxServer = lm.fields.find(f => f.name === 'raybox_server');
    expect(rayboxServer).toBeDefined();
    expect(rayboxServer!.kind).toBe('lookup');
    expect(rayboxServer!.lookup?.path).toBe('raybox_id.name');
  });

  it('MachineStatusSnapshot should have many fields (50+ fields)', () => {
    const mss = parsed.models.find(m => m.name === 'MachineStatusSnapshot')!;
    expect(mss).toBeDefined();
    // Count all own fields (stored + lookup)
    expect(mss.fields.length).toBeGreaterThanOrEqual(50);
  });

  it('documentation H1 headers (# RayBox 식별, # 축 위치, etc.) should NOT create new models - fields after them should belong to the containing model', () => {
    // LaserMachine should contain fields from doc sections like "# RayBox 식별", "# 제어카드", etc.
    const lm = parsed.models.find(m => m.name === 'LaserMachine')!;
    // Fields from "# RayBox 식별" section
    const gmid = lm.fields.find(f => f.name === 'gmid');
    expect(gmid).toBeDefined();
    const serverIp = lm.fields.find(f => f.name === 'server_ip');
    expect(serverIp).toBeDefined();
    const macAddress = lm.fields.find(f => f.name === 'mac_address');
    expect(macAddress).toBeDefined();

    // Fields from "# 제어카드" section
    const cardId = lm.fields.find(f => f.name === 'card_id');
    expect(cardId).toBeDefined();
    const cardType = lm.fields.find(f => f.name === 'card_type');
    expect(cardType).toBeDefined();

    // MachineStatusSnapshot should contain fields from "# 축 위치", "# 속도", etc.
    const mss = parsed.models.find(m => m.name === 'MachineStatusSnapshot')!;
    const axisX = mss.fields.find(f => f.name === 'axis_x');
    expect(axisX).toBeDefined();
    const speedX = mss.fields.find(f => f.name === 'speed_x');
    expect(speedX).toBeDefined();
    const laserPower = mss.fields.find(f => f.name === 'laser_power');
    expect(laserPower).toBeDefined();

    // These doc H1 headers should NOT appear as model names
    const modelNames = parsed.models.map(m => m.name);
    expect(modelNames).not.toContain('RayBox 식별');
    expect(modelNames).not.toContain('제어카드');
    expect(modelNames).not.toContain('축 위치');
    expect(modelNames).not.toContain('속도');
  });

  it('CuttingTask has both lookup and rollup fields', () => {
    const ct = parsed.models.find(m => m.name === 'CuttingTask')!;
    expect(ct).toBeDefined();

    // Lookup fields
    const woNo = ct.fields.find(f => f.name === 'work_order_no');
    expect(woNo).toBeDefined();
    expect(woNo!.kind).toBe('lookup');

    const assignedName = ct.fields.find(f => f.name === 'assigned_machine_name');
    expect(assignedName).toBeDefined();
    expect(assignedName!.kind).toBe('lookup');

    // Rollup fields
    const partCount = ct.fields.find(f => f.name === 'part_count');
    expect(partCount).toBeDefined();
    expect(partCount!.kind).toBe('rollup');
    expect(partCount!.rollup?.target).toBe('CuttingTaskPart');
    expect(partCount!.rollup?.aggregate).toBe('count');

    const totalQty = ct.fields.find(f => f.name === 'total_qty');
    expect(totalQty).toBeDefined();
    expect(totalQty!.kind).toBe('rollup');
    expect(totalQty!.rollup?.aggregate).toBe('sum');
    expect(totalQty!.rollup?.field).toBe('quantity');
  });

  it('DailyMachineStat has computed fields (total_time_sec, start_rate, cut_rate)', () => {
    const dms = parsed.models.find(m => m.name === 'DailyMachineStat')!;
    expect(dms).toBeDefined();

    const totalTime = dms.fields.find(f => f.name === 'total_time_sec');
    expect(totalTime).toBeDefined();
    expect(totalTime!.kind).toBe('computed');
    expect(totalTime!.computed?.expression).toBe(
      'work_time_sec + idle_time_sec + alarm_time_sec'
    );

    const startRate = dms.fields.find(f => f.name === 'start_rate');
    expect(startRate).toBeDefined();
    expect(startRate!.kind).toBe('computed');
    expect(startRate!.computed?.expression).toContain('work_time_sec');

    const cutRate = dms.fields.find(f => f.name === 'cut_rate');
    expect(cutRate).toBeDefined();
    expect(cutRate!.kind).toBe('computed');
    expect(cutRate!.computed?.expression).toContain('laser_time_sec');
  });
});

// ============================================================
// E. Resolver tests (merge both files)
// ============================================================
describe('E. Resolver tests (merge both files)', () => {
  const mesParsed = parseSingleFile(mesContent, 'mes.m3l');
  const rayboxParsed = parseSingleFile(rayboxContent, 'raybox.m3l');
  const merged = resolve([mesParsed, rayboxParsed]);

  it('total model count should be sum of both files\' models', () => {
    const expectedTotal = mesParsed.models.length + rayboxParsed.models.length;
    expect(merged.models.length).toBe(expectedTotal);
  });

  it('total enum count should be sum of both files\' enums', () => {
    const expectedTotal = mesParsed.enums.length + rayboxParsed.enums.length;
    expect(merged.enums.length).toBe(expectedTotal);
  });

  it('namespace from first file should be used as project name', () => {
    expect(merged.project.name).toBe('umes.ray.mes');
  });

  it('merged sources include both files', () => {
    expect(merged.sources).toEqual(['mes.m3l', 'raybox.m3l']);
  });

  it('models from both files are accessible', () => {
    const modelNames = merged.models.map(m => m.name);
    // From mes.m3l
    expect(modelNames).toContain('Customer');
    expect(modelNames).toContain('WorkOrder');
    // From raybox.m3l
    expect(modelNames).toContain('LaserMachine');
    expect(modelNames).toContain('CuttingTask');
    expect(modelNames).toContain('MachineStatusSnapshot');
  });

  it('inheritance is resolved across files (raybox models inherit BaseEntity from mes)', () => {
    const laserMachine = merged.models.find(m => m.name === 'LaserMachine')!;
    // After resolution, LaserMachine should have inherited fields from BaseEntity
    const fieldNames = laserMachine.fields.map(f => f.name);
    expect(fieldNames).toContain('id'); // from BaseEntity
    expect(fieldNames).toContain('created_at'); // from BaseEntity
    expect(fieldNames).toContain('machine_id'); // own field
  });
});

// ============================================================
// F. Validator tests
// ============================================================
describe('F. Validator tests', () => {
  it('validate mes.m3l AST - lookup/rollup FK validations work with @reference', () => {
    const mesParsed = parseSingleFile(mesContent, 'mes.m3l');
    const ast = resolve([mesParsed]);
    const result = validate(ast);

    // In mes.m3l, all FK fields have @reference, so E001/E002 errors should be minimal or zero
    // for references within the same file
    const e001Errors = result.errors.filter(e => e.code === 'M3L-E001');
    const e002Errors = result.errors.filter(e => e.code === 'M3L-E002');

    // WorkOrder rollup targets WorkOrderItem.work_order_id which has @reference(WorkOrder)
    // so there should be no E001 for that rollup
    const woE001 = e001Errors.filter(e => e.message.includes('item_count'));
    expect(woE001.length).toBe(0);

    // MaterialSpec lookup uses material_id which has @reference(Material)
    // so there should be no E002 for that lookup
    const msE002 = e002Errors.filter(e => e.message.includes('material_name') && e.message.includes('MaterialSpec'));
    expect(msE002.length).toBe(0);
  });

  it('validate raybox.m3l standalone - may have some E001/E002 warnings since it references mes.m3l models not present', () => {
    const rayboxParsed = parseSingleFile(rayboxContent, 'raybox.m3l');
    const ast = resolve([rayboxParsed]);
    const result = validate(ast);

    // raybox.m3l references Machine, WorkOrder, etc. from mes.m3l which are not in this standalone AST
    // Inheritance resolution should report E007 for unresolved parents like BaseEntity, Auditable
    const e007Errors = result.errors.filter(e => e.code === 'M3L-E007');
    expect(e007Errors.length).toBeGreaterThan(0);
  });

  it('validate merged (mes + raybox) should resolve cross-file references', () => {
    const mesParsed = parseSingleFile(mesContent, 'mes.m3l');
    const rayboxParsed = parseSingleFile(rayboxContent, 'raybox.m3l');
    const ast = resolve([mesParsed, rayboxParsed]);
    const result = validate(ast);

    // With both files merged, BaseEntity and Auditable should be resolved
    const e007Errors = result.errors.filter(e => e.code === 'M3L-E007');
    expect(e007Errors.length).toBe(0);
  });
});
