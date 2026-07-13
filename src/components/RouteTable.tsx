import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import type { Route } from "@/lib/api"

interface RouteTableProps {
  routes: Route[]
  onEdit: (index: number) => void
  onDelete: (index: number) => void
}

export function RouteTable({ routes, onEdit, onDelete }: RouteTableProps) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-20">端口</TableHead>
          <TableHead className="w-32">名称</TableHead>
          <TableHead>Shell 路径</TableHead>
          <TableHead className="w-20">默认</TableHead>
          <TableHead className="w-32">操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {routes.map((route, index) => (
          <TableRow key={index}>
            <TableCell>{route.port}</TableCell>
            <TableCell>{route.name}</TableCell>
            <TableCell className="font-mono text-sm">{route.shell}</TableCell>
            <TableCell>
              <Checkbox checked={route.default} disabled />
            </TableCell>
            <TableCell>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={() => onEdit(index)}>
                  编辑
                </Button>
                <Button variant="outline" size="sm" onClick={() => onDelete(index)}>
                  删除
                </Button>
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
