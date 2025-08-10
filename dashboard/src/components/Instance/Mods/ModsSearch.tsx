import React, { useState } from 'react';
import { searchMods, installMod } from '../../../utils/apis';

const ModsSearch: React.FC<{ uuid?: string }> = ({ uuid }) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  const handleSearch = async () => {
    setLoading(true);
    const data = await searchMods(query);
    setResults(data);
    setLoading(false);
  };

  const handleInstall = async (projectId: string) => {
    if (!uuid) return;
    await installMod(uuid, { project_id: projectId });
    alert('Installed!'); // Replace with toast
  };

  return (
    <div>
      <div className="flex gap-2 mb-2">
        <input
          className="border px-2 py-1 rounded"
          value={query}
          onChange={e => setQuery(e.target.value)}
          placeholder="Search mods…"
        />
        <button className="btn btn-primary" onClick={handleSearch} disabled={loading}>
          Search
        </button>
      </div>
      {loading && <div>Loading…</div>}
      <div>
        {results.map((mod) => (
          <div key={mod.id} className="border-b py-2 flex justify-between items-center">
            <div>
              <div className="font-bold">{mod.title}</div>
              <div className="text-xs text-gray-600">{mod.description}</div>
            </div>
            <button className="btn btn-xs btn-success" onClick={() => handleInstall(mod.id)}>
              Install
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ModsSearch;