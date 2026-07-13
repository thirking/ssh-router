import { useState, useEffect } from "react"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Checkbox } from "@/components/ui/checkbox"
import { Button } from "@/components/ui/button"
import type { Route } from "@/lib/api"

interface RouteDialogProps {
  open: boolean
  route: Route | null
  onSave: (route: Route) => void
  onClose: () => void
}

const emptyRoute: Route = {
  port: 0,
  name: "",
  shell: "",
  interactiveTemplate: "",
  commandTemplate: "",
  tmpFileExt: ".sh",
  default: false,
}

export function RouteDialog({ open, route, onSave, onClose }: RouteDialogProps) {
  const [form, setForm] = useState<Route>(emptyRoute)

  useEffect(() => {
    setForm(route ?? emptyRoute)
  }, [route, open])

  const handleChange = (field: keyof Route, value: string | number | boolean) => {
    setForm(prev => ({ ...prev, [field]: value }))
  }

  const handleSave = () => {
    onSave(form)
    onClose()
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{route ? "编辑路由" : "添加路由"}</DialogTitle>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="port">端口</Label>
              <Input
                id="port"
                type="number"
                value={form.port || ""}
                onChange={e => handleChange("port", parseInt(e.target.value) || 0)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="name">名称</Label>
              <Input
                id="name"
                value={form.name}
                onChange={e => handleChange("name", e.target.value)}
              />
            </div>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="shell">Shell 路径</Label>
            <Input
              id="shell"
              value={form.shell}
              onChange={e => handleChange("shell", e.target.value)}
              placeholder="C:\Program Files\PowerShell\7\pwsh.exe"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="interactiveTemplate">交互式模板</Label>
            <Input
              id="interactiveTemplate"
              value={form.interactiveTemplate}
              onChange={e => handleChange("interactiveTemplate", e.target.value)}
              placeholder="&quot;{shell}&quot; -l"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="commandTemplate">命令模板</Label>
            <Input
              id="commandTemplate"
              value={form.commandTemplate}
              onChange={e => handleChange("commandTemplate", e.target.value)}
              placeholder="&quot;{shell}&quot; -File &quot;{tmpfile}&quot;"
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="tmpFileExt">临时文件扩展名</Label>
              <Input
                id="tmpFileExt"
                value={form.tmpFileExt}
                onChange={e => handleChange("tmpFileExt", e.target.value)}
                placeholder=".sh"
              />
            </div>
            <div className="grid gap-2 items-end">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="default"
                  checked={form.default}
                  onCheckedChange={checked => handleChange("default", checked === true)}
                />
                <Label htmlFor="default">设为默认路由</Label>
              </div>
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>取消</Button>
          <Button onClick={handleSave}>保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
