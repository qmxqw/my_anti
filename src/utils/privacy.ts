const PRIVACY_MODE_STORAGE_KEY = 'agtools.privacy_mode_enabled'
export const PRIVACY_MODE_CHANGED_EVENT = 'agtools:privacy-mode-changed'



function maskEmail(value: string): string {
  const [localPart = '', domainPart = ''] = value.split('@')

  const rawLocal = localPart.trim()
  let localMasked = rawLocal
  if (rawLocal) {
    if (rawLocal.length > 3) {
      localMasked = `${rawLocal.slice(0, 2)}...${rawLocal.slice(-1)}`
    } else if (rawLocal.length > 1) {
      localMasked = `${rawLocal.slice(0, 1)}...${rawLocal.slice(-1)}`
    } else {
      localMasked = `${rawLocal}...`
    }
  }

  const rawDomain = domainPart.trim()
  if (!rawDomain) return localMasked ? `${localMasked}@` : value

  return `${localMasked}@${rawDomain.slice(0, 2)}`
}

function maskGeneric(value: string): string {
  const raw = value.trim()
  if (!raw) return raw
  if (raw.length <= 3) return `${raw.charAt(0)}**`
  if (raw.length <= 6) return `${raw.slice(0, 1)}***${raw.slice(-1)}`
  if (raw.length <= 10) return `${raw.slice(0, 2)}***${raw.slice(-2)}`
  return `${raw.slice(0, 3)}***${raw.slice(-3)}`
}

export function isPrivacyModeEnabledByDefault(): boolean {
  try {
    return localStorage.getItem(PRIVACY_MODE_STORAGE_KEY) === '1'
  } catch {
    return false
  }
}

export function persistPrivacyModeEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(PRIVACY_MODE_STORAGE_KEY, enabled ? '1' : '0')
    window.dispatchEvent(new CustomEvent(PRIVACY_MODE_CHANGED_EVENT, { detail: enabled }))
  } catch {
    // ignore localStorage write failures
  }
}

export function maskSensitiveValue(value: string | null | undefined, enabled: boolean): string {
  const raw = (value ?? '').trim()
  if (!raw || !enabled) return raw
  if (raw.includes('@')) return maskEmail(raw)
  return maskGeneric(raw)
}
