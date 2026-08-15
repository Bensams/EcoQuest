import type { ReactNode } from 'react';
import type { ColumnProps } from 'react-aria-components';
import { Cell, Column, Row, Table as AriaTable, TableBody, TableHeader } from 'react-aria-components';
import { cn } from '../../lib/utils';

export type TableColumn<T> = {
  key: string | number;
  header: ReactNode;
  isRowHeader?: boolean;
  className?: string;
  cellClassName?: (row: T) => string;
} & ColumnProps;

const tableCn = 'w-full border-collapse text-sm text-forest';
const cellCn = 'border-t border-sage py-2.5 pr-4 text-left align-top pl-4';
const headCellCn =
  'py-2 pr-4 text-xs font-medium uppercase tracking-wide text-forest-muted text-left align-bottom pl-4';

export function DataTable<T extends { id: string | number }>({
  columns,
  rows,
  renderCell,
  rowKey = (r) => r.id,
  className,
  'aria-label': ariaLabel,
}: {
  columns: TableColumn<T>[];
  rows: readonly T[] | null | undefined;
  renderCell: (row: T, columnKey: string | number) => ReactNode;
  rowKey?: (row: T) => string | number;
  className?: string;
  'aria-label'?: string;
}) {
  const body = rows ?? [];
  return (
    <div
      className={cn(
        'overflow-x-auto rounded-lg border border-sage bg-surface',
        className,
      )}
    >
      <AriaTable
        data-component="data-table"
        aria-label={ariaLabel}
        className={tableCn}
      >
        <TableHeader columns={columns}>
          {(col) => (
            <Column
              id={col.key}
              isRowHeader={col.isRowHeader}
              className={cn(headCellCn)}
            >
              <span className={col.className}>{col.header}</span>
            </Column>
          )}
        </TableHeader>
        <TableBody items={body}>
          {(row) => (
            <Row id={rowKey(row)} columns={columns}>
              {(col) => (
                <Cell
                  data-component="data-table-cell"
                  className={cn(
                    cellCn,
                    col.isRowHeader ? 'font-medium' : '',
                    col.cellClassName ? col.cellClassName(row) : '',
                  )}
                >
                  {renderCell(row, col.key)}
                </Cell>
              )}
            </Row>
          )}
        </TableBody>
      </AriaTable>
    </div>
  );
}
