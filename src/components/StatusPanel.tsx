import { CheckCircle2, XCircle, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import type { Status } from "@/lib/api"

interface StatusPanelProps {
  status: Status | null
  loading: boolean
  onRefresh: () => void
}

function StatusItem({ ok, label, detail }: { ok: boolean; label: string; detail?: string }) {
  return (
    <div className="flex items-center gap-2">
      {ok ? (
        <CheckCircle2 className="h-4 w-4 text-green-500" />
      ) : (
        <XCircle className="h-4 w-4 text-red-500" />
      )}
      <span className="text-sm">{label}</span>
      {detail && <span className="text-xs text-muted-foreground truncate max-w-64">{detail}</span>}
    </div>
  )
}

export function StatusPanel({ status, loading, onRefresh }: StatusPanelProps) {
  return (
    <div className="rounded-lg border p-4 mb-4">
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-lg font-semibold">安装状态</h2>
        <Button variant="outline" size="sm" onClick={onRefresh} disabled={loading}>
          <RefreshCw className={`h-4 w-4 mr-1 ${loading ? "animate-spin" : ""}`} />
          刷新
        </Button>
      </div>
      {status ? (
        <div className="grid grid-cols-2 gap-2">
          <StatusItem
            ok={status.cliDeployed}
            label="CLI 已部署"
            detail={status.cliDeployed ? status.cliPath : undefined}
          />
          <StatusItem
            ok={status.defaultShellSet}
            label="DefaultShell 已设置"
            detail={status.defaultShellSet ? status.defaultShellValue : undefined}
          />
          <StatusItem
            ok={status.configExists}
            label="配置文件存在"
          />
          <StatusItem
            ok={status.sshdRunning}
            label="sshd 服务"
            detail={status.sshdStatus}
          />
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">
          {loading ? "检查中..." : "点击刷新检查状态"}
        </p>
      )}
    </div>
  )
}
