import { Tab, TabList, Tabs as AriaTabs } from 'react-aria-components';
import { cn } from '../../lib/utils';

export function Tabs<T extends string>({
  items,
  active,
  onChange,
  className,
}: {
  items: Array<{ key: T; label: string }>;
  active: T;
  onChange: (key: T) => void;
  className?: string;
}) {
  return (
    <AriaTabs
      data-component="tabs"
      selectedKey={active}
      onSelectionChange={(key) => onChange(String(key) as T)}
      className={className}
    >
      <TabList className="flex gap-1 overflow-x-auto border-b border-sage">
        {items.map((item) => (
          <Tab
            key={item.key}
            id={item.key}
            className={({ isSelected }) =>
              cn(
                'whitespace-nowrap rounded-t-lg border-b-2 px-3 py-2 text-sm font-medium transition-colors',
                isSelected ? 'border-leaf text-leaf' : 'border-transparent text-forest-muted hover:text-forest',
              )
            }
          >
            {item.label}
          </Tab>
        ))}
      </TabList>
    </AriaTabs>
  );
}
