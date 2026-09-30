/**
 * 字体家族设置：纯族名存取 + 本机字体枚举（搜索面板 FontSelect.vue 的逻辑层）。
 *
 * 持久化的是**纯族名**（如 `Microsoft YaHei`），不是完整 CSS 值——应用时由
 * buildFontFamilyValue 拼上默认栈，字体被卸载/值失效时自动回落，不破版；
 * 空串 = 跟随系统默认栈。
 *
 * 本机字体枚举用 Local Font Access API（`window.queryLocalFonts()`，Chromium 104+）：
 * cdown 只跑 Windows WebView2，该 API 可用。两个硬约束：
 * - 需要 secure context + **用户手势**（调用必须挂在「点开字体下拉」上，
 *   挂载即调可能连授权弹窗都弹不出来）；
 * - 拒授权/不支持/返回空列表时回退常用字体表，功能不残废。
 */

/** 默认字体栈：未设置字体家族、或设置值失效时的回退；与 style.css 的 :root 保持一致 */
export const DEFAULT_FONT_STACK =
  'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Microsoft YaHei", Roboto, Arial, sans-serif';

/** 家族名长度上限：字体名不会很长，超长视为异常值 */
const FONT_FAMILY_MAX_LEN = 64;

/** 回退候选：无法枚举本机字体家族时至少给出常用中文字体 */
export const FALLBACK_FONT_FAMILIES = [
  "Microsoft YaHei",
  "Microsoft YaHei UI",
  "SimHei",
  "SimSun",
  "KaiTi",
  "Segoe UI",
  "Arial",
];

/**
 * 字体家族名清洗：先剥掉引号，再取开头的合法片段（中英文、数字与 `.` `_` `-` 空格），
 * 遇到第一个非法字符即截断，并限长。
 * 值来自本机字体名与设置回写，不清洗的话一个 `;` 就能把整条 font-family 打崩；
 * 「截断」而非「剔除非法字符」是为了不留拼接出的假名字（`"Arial";color:red` → `Arial`）。
 */
export function sanitizeFontFamily(name: string): string {
  const unquoted = name.replace(/["']/g, "").trim();
  const lead = unquoted.match(/^[\p{L}\p{N} ._-]+/u);
  return (lead?.[0] ?? "").trim().slice(0, FONT_FAMILY_MAX_LEN);
}

/** 拼出 CSS font-family 值：用户字体在前，默认栈兜底（字体被卸载也不破版）；空值回落默认栈 */
export function buildFontFamilyValue(family: string): string {
  const safe = sanitizeFontFamily(family);
  return safe ? `"${safe}", ${DEFAULT_FONT_STACK}` : DEFAULT_FONT_STACK;
}

/**
 * 显示名：有中文映射 → `中文名 (English)`，否则原样。
 * 映射来自 `<数据目录>/font-family-map.toml`（后端 getFontFamilyMap）。
 */
export function displayFontFamily(family: string, map: Record<string, string>): string {
  const cn = family ? map[family] : "";
  return cn ? `${cn} (${family})` : family;
}

/**
 * 搜索文本：中文名与英文族名都要能被搜到，
 * 否则用户搜「雅黑」搜不到 `Microsoft YaHei`。
 */
export function fontFamilySearchText(family: string, map: Record<string, string>): string {
  const cn = map[family];
  return cn ? `${family} ${cn}` : family;
}

/**
 * 列表渲染窗口：有选中项时把窗口挪到它周围（保留字母序），否则从头开始。
 * 列表只渲染 `max` 项，选中项若在窗口外根本不在 DOM 里，滚动定位也就无从谈起。
 * `offset` 是选中项上方保留的上下文项数；选中项不存在时从 0 开始。
 */
export function fontListWindow(
  all: string[],
  selected: string,
  max: number,
  offset: number,
): string[] {
  const idx = selected ? all.indexOf(selected) : -1;
  const desired = idx > offset ? idx - offset : 0;
  // 再夹一次：窗口尽量填满，避免选中项靠近末尾时右侧留白、前面白白被裁掉
  const start = Math.min(desired, Math.max(0, all.length - max));
  return all.slice(start, start + max);
}

// —— 本机字体家族枚举 ——

/** queryLocalFonts 只取用得到的 family（还有 fullName / postscriptName / style） */
interface LocalFont {
  family: string;
}

type FontAwareWindow = Window & {
  queryLocalFonts?: () => Promise<LocalFont[]>;
};

/**
 * 去重 + 排序：queryLocalFonts 返回的是逐 style（Regular/Bold/Italic…），
 * 同一 family 会出现多次，必须按 family 去重。
 */
function normalizeFontList(list: LocalFont[]): string[] {
  const families = new Set<string>();
  for (const font of list) {
    const name = sanitizeFontFamily(font?.family ?? "");
    if (name) families.add(name);
  }
  return [...families].sort((a, b) => a.localeCompare(b, "zh-Hans-CN"));
}

/**
 * 读取结果状态：
 * - `ok`：枚举成功（拿到 >0 项）；
 * - `unsupported`：环境无此 API（内核过旧）；
 * - `unreadable`：其余一切读不到的情况——抛异常、返回空数组、权限被拒绝。
 *
 * WebView2 里拒绝不一定表现为 NotAllowedError（实测也可能返回空数组或抛别的异常），
 * navigator.permissions 又未必支持 local-fonts 这个名字，判定不可靠；用户能采取的
 * 行动只有一种——退出应用后删掉 WebView profile（EBWebView）再重启重新授权。
 */
export type FontListStatus = "ok" | "unsupported" | "unreadable";

export interface SystemFontsResult {
  families: string[];
  status: FontListStatus;
  /** 失败原因摘要（status 非 ok 时给出）：界面小字显示 + 日志排查用 */
  detail: string;
}

/** 读取本机字体家族名；失败时返回回退候选表并说明原因（供界面提示，不静默） */
export async function loadSystemFonts(): Promise<SystemFontsResult> {
  const fallback = async (detail: string): Promise<SystemFontsResult> => ({
    families: [...FALLBACK_FONT_FAMILIES],
    status: "unreadable",
    detail: `${detail}；权限状态=${await fontPermissionState()}`,
  });

  const w = globalThis.window as FontAwareWindow | undefined;
  if (!w?.queryLocalFonts) {
    return {
      families: [...FALLBACK_FONT_FAMILIES],
      status: "unsupported",
      detail: "window.queryLocalFonts 不存在",
    };
  }
  try {
    const families = normalizeFontList(await w.queryLocalFonts());
    if (families.length) return { families, status: "ok", detail: "" };
    // 空数组：WebView2 上「权限被拒」就可能表现成这样，故按读不到处理
    return fallback("枚举到 0 个字体家族");
  } catch (e) {
    const err = e as { name?: string; message?: string };
    return fallback(`${err?.name ?? "Error"}: ${err?.message ?? String(e)}`);
  }
}

/** 查询 local-fonts 权限状态；Permissions API 不支持该名字时返回失败原因（仅用于排查） */
async function fontPermissionState(): Promise<string> {
  try {
    const status = await navigator.permissions.query({ name: "local-fonts" as PermissionName });
    return status.state;
  } catch (e) {
    return `查询失败(${(e as { name?: string })?.name ?? "Error"})`;
  }
}
