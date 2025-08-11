import React, { useState, useMemo } from 'react';
import { useEventStream } from 'data/EventStream';
import { EventInner } from 'bindings/EventInner';
import { EventType } from 'bindings/EventType';
import { InstanceContext } from 'data/InstanceContext';
import { FixedSizeList as List } from 'react-window';

const EVENT_TYPE_LABELS: Record<string, string> = {
  InstanceEvent: 'Instance',
  UserEvent: 'User',
  MacroEvent: 'Macro',
  PlayitggRunnerEvent: 'Playit',
  ProgressionEvent: 'Progression',
  FSEvent: 'FS',
};

const EventViewer: React.FC = () => {
  const { allEvents } = useEventStream(); // buffered + live events
  const [typeFilter, setTypeFilter] = useState<string>('');
  const [instanceFilter, setInstanceFilter] = useState<string>('');
  const [search, setSearch] = useState('');

  const filteredEvents = useMemo(() => {
    return allEvents.filter(ev => {
      if (typeFilter && ev.event_inner.type !== typeFilter) return false;
      if (instanceFilter && ev.instance_uuid && ev.instance_uuid !== instanceFilter) return false;
      if (search) {
        const s = search.toLowerCase();
        if (!(`${ev.details ?? ''} ${ev.event_inner.message ?? ''}`.toLowerCase().includes(s)))
          return false;
      }
      return true;
    });
  }, [allEvents, typeFilter, instanceFilter, search]);

  // Gather unique instance UUIDs for filter dropdown
  const instanceUuids = useMemo(
    () =>
      Array.from(new Set(allEvents.map(ev => ev.instance_uuid).filter(Boolean))),
    [allEvents]
  );

  return (
    <div className="max-w-5xl mx-auto p-6">
      <h1 className="text-2xl font-bold mb-4">Event Viewer</h1>
      <div className="flex gap-4 mb-4">
        <select value={typeFilter} onChange={e => setTypeFilter(e.target.value)} className="border px-2 py-1 rounded">
          <option value="">All Types</option>
          {Object.entries(EVENT_TYPE_LABELS).map(([type, label]) => (
            <option key={type} value={type}>{label}</option>
          ))}
        </select>
        <select value={instanceFilter} onChange={e => setInstanceFilter(e.target.value)} className="border px-2 py-1 rounded">
          <option value="">All Instances</option>
          {instanceUuids.map(uuid => (
            <option key={uuid} value={uuid}>{uuid}</option>
          ))}
        </select>
        <input
          className="border px-2 py-1 rounded"
          placeholder="Search details/message"
          value={search}
          onChange={e => setSearch(e.target.value)}
        />
      </div>
      <div style={{ height: '60vh', border: '1px solid #eee', borderRadius: 6, background: 'white' }}>
        <List
          height={400}
          itemCount={filteredEvents.length}
          itemSize={64}
          width="100%"
        >
          {({ index, style }) => {
            const ev = filteredEvents[index];
            return (
              <div style={style} className="px-4 py-2 border-b flex flex-col">
                <div className="text-xs text-gray-400 flex items-center gap-2">
                  <span>{new Date(ev.timestamp * 1000).toLocaleString()}</span>
                  <span className="inline-block px-2 py-0.5 bg-gray-200 rounded text-xs">{EVENT_TYPE_LABELS[ev.event_inner.type] || ev.event_inner.type}</span>
                  {ev.instance_uuid && <span className="font-mono text-gray-500">Instance: {ev.instance_uuid}</span>}
                </div>
                <div className="font-semibold">{ev.details ?? ev.event_inner.message}</div>
                <div className="text-xs text-gray-600">{ev.event_inner.message}</div>
              </div>
            );
          }}
        </List>
      </div>
    </div>
  );
};

export default EventViewer;