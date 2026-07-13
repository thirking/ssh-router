import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

interface SftpFieldProps {
  value: string
  onChange: (value: string) => void
}

export function SftpField({ value, onChange }: SftpFieldProps) {
  return (
    <div className="grid gap-2">
      <Label htmlFor="sftpCommand">SFTP 命令</Label>
      <Input
        id="sftpCommand"
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder="cmd.exe /c sftp-server.exe"
      />
    </div>
  )
}
