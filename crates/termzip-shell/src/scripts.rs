//! Bundled shell-integration scripts.

use std::fs;
use std::path::{Path, PathBuf};

use crate::launch::ShellKind;

pub const ZSH_SCRIPT: &str = r#"# TermZip shell integration (zsh)
if [[ -o interactive ]]; then
  autoload -Uz add-zsh-hook
  __termzip_emit() { printf '\e]%s\a' "$1" }
  __termzip_started=0
  __termzip_precmd() {
    local exit_code=$?
    if (( __termzip_started )); then
      __termzip_emit "133;D;$exit_code"
      __termzip_started=0
    fi
    __termzip_emit "133;A"
    __termzip_emit "1337;CurrentDir=$PWD"
  }
  __termzip_preexec() {
    __termzip_started=1
    __termzip_emit "133;B"
    __termzip_emit "133;C"
    if (( $+commands[base64] )); then
      local encoded
      encoded=$(printf %s "$1" | base64 | tr -d '\n')
      __termzip_emit "1337;CommandB64=$encoded"
    else
      local clean=${1//[[:cntrl:]]/ }
      __termzip_emit "1337;Command=$clean"
    fi
  }
  add-zsh-hook precmd __termzip_precmd
  add-zsh-hook preexec __termzip_preexec
fi
"#;

pub const BASH_SCRIPT: &str = r#"# TermZip shell integration (bash)
__termzip_emit() { printf '\e]%s\a' "$1"; }
__termzip_started=0
__termzip_precmd() {
  local status=$?
  if [ "$__termzip_started" = 1 ]; then
    __termzip_emit "133;D;$status"
    __termzip_started=0
  fi
  __termzip_emit "133;A"
  __termzip_emit "1337;CurrentDir=$PWD"
}
__termzip_preexec() {
  case "$BASH_COMMAND" in
    __termzip_*|PROMPT_COMMAND*) return ;;
  esac
  [ "$__termzip_started" = 1 ] && return
  __termzip_started=1
  __termzip_emit "133;B"
  __termzip_emit "133;C"
  if command -v base64 >/dev/null 2>&1; then
    __termzip_emit "1337;CommandB64=$(printf %s "$BASH_COMMAND" | base64 | tr -d '\n')"
  else
    __termzip_emit "1337;Command=$BASH_COMMAND"
  fi
}
trap '__termzip_preexec' DEBUG
PROMPT_COMMAND="__termzip_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
"#;

pub const FISH_SCRIPT: &str = r#"# TermZip shell integration (fish)
function __termzip_emit
    printf '\e]%s\a' $argv[1]
end

function __termzip_on_preexec --on-event fish_preexec
    __termzip_emit "133;B"
    __termzip_emit "133;C"
    if type -q base64
        __termzip_emit "1337;CommandB64="(printf %s "$argv[1]" | base64 | tr -d '\n')
    else
        __termzip_emit "1337;Command=$argv[1]"
    end
end

function __termzip_on_postexec --on-event fish_postexec
    set -l status_code $status
    __termzip_emit "133;D;$status_code"
end

function __termzip_on_prompt --on-event fish_prompt
    __termzip_emit "133;A"
    __termzip_emit "1337;CurrentDir=$PWD"
end
"#;

pub const POWERSHELL_SCRIPT: &str = r#"# TermZip shell integration (PowerShell)
$esc = [char]27
$bel = [char]7
function __TermZip-Emit([string]$Body) {
    [Console]::Write("$esc]$Body$bel")
}
function global:prompt {
    $ok = $?
    $code = if ($LASTEXITCODE -ne $null -and $LASTEXITCODE -ne 0) { $LASTEXITCODE }
            elseif ($ok) { 0 } else { 1 }
    if ($Global:__TermZipStarted) {
        __TermZip-Emit "133;D;$code"
        $Global:__TermZipStarted = $false
        $last = Get-History -Count 1
        if ($last -and $Global:__TermZipLastId -ne $last.Id) {
            $Global:__TermZipLastId = $last.Id
            $bytes = [Text.Encoding]::UTF8.GetBytes($last.CommandLine)
            __TermZip-Emit ("1337;CommandB64=" + [Convert]::ToBase64String($bytes))
        }
    }
    __TermZip-Emit "133;A"
    __TermZip-Emit ("1337;CurrentDir=" + (Get-Location).Path)
    if ($Global:__TermZipOriginalPrompt) {
        & $Global:__TermZipOriginalPrompt
    } else {
        "PS> "
    }
}
$Global:__TermZipStarted = $false
$Global:__TermZipOriginalPrompt = $function:prompt
$Global:__TermZipLastId = -1
# Re-arm the exec flag whenever a native command is about to run.
$ExecutionContext.InvokeCommand.PreCommandLookupAction = {
    param($CommandName, $CommandLookupEventArgs)
    $Global:__TermZipStarted = $true
}
"#;

/// Write the integration script for `kind` into `dir`, returning its path.
pub fn ensure_script(kind: ShellKind, dir: &Path) -> Option<PathBuf> {
    let file_name = match kind {
        ShellKind::Zsh => "termzip-integration.zsh",
        ShellKind::Bash => "termzip-integration.bash",
        ShellKind::Fish => "termzip-integration.fish",
        ShellKind::PowerShell => "termzip-integration.ps1",
        ShellKind::Cmd | ShellKind::Other => return None,
    };

    fs::create_dir_all(dir).ok()?;
    let path = dir.join(file_name);
    let contents = script_for(kind)?;
    if fs::read_to_string(&path).is_ok_and(|existing| existing == contents) {
        return Some(path);
    }
    fs::write(&path, contents).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
    }
    Some(path)
}

pub fn script_for(kind: ShellKind) -> Option<&'static str> {
    match kind {
        ShellKind::Zsh => Some(ZSH_SCRIPT),
        ShellKind::Bash => Some(BASH_SCRIPT),
        ShellKind::Fish => Some(FISH_SCRIPT),
        ShellKind::PowerShell => Some(POWERSHELL_SCRIPT),
        ShellKind::Cmd | ShellKind::Other => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ShellKind, ensure_script, script_for};

    #[test]
    fn every_integrated_shell_has_a_script() {
        assert!(script_for(ShellKind::Zsh).unwrap().contains("133;A"));
        assert!(script_for(ShellKind::Bash).unwrap().contains("133;D"));
        assert!(script_for(ShellKind::Fish).unwrap().contains("133;C"));
        assert!(script_for(ShellKind::PowerShell).unwrap().contains("133;D"));
        assert!(script_for(ShellKind::Cmd).is_none());
    }

    #[test]
    fn scripts_are_written_once_and_stable() {
        let dir = std::env::temp_dir().join(format!("termzip-scripts-{}", std::process::id()));
        let first = ensure_script(ShellKind::Zsh, &dir).unwrap();
        let contents = std::fs::read_to_string(&first).unwrap();
        let second = ensure_script(ShellKind::Zsh, &dir).unwrap();

        assert_eq!(first, second);
        assert_eq!(std::fs::read_to_string(second).unwrap(), contents);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
