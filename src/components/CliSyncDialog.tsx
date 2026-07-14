import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

interface CliSyncDialogProps {
  open: boolean
  syncing: boolean
  onOpenChange: (open: boolean) => void
  onSync: () => void
}

export function CliSyncDialog({
  open,
  syncing,
  onOpenChange,
  onSync,
}: CliSyncDialogProps) {
  return (
    <Dialog open={open} onOpenChange={syncing ? undefined : onOpenChange}>
      <DialogContent showCloseButton={!syncing}>
        <DialogHeader>
          <DialogTitle>需要同步路由 CLI</DialogTitle>
          <DialogDescription>
            GUI 已更新，但系统中部署的 CLI 仍是旧版本。同步需要管理员权限，不会修改现有路由配置。
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={syncing}>
            稍后
          </Button>
          <Button onClick={onSync} disabled={syncing}>
            {syncing ? "同步中..." : "同步 CLI"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
