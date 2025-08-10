import React, { useEffect, useState } from 'react';
import { listInstalledMods, uninstallMod } from '../../../utils/apis';

const ModsInstalled: React.FC<{ uuid?: string }> = ({ uuid }) => {
  const [installed, setInstalled] = useState<any[]>([]);

  useEffect(() => {
    if (!uuid) return;
    listInstalledMods(uuid).then(setInstalled);
  }, [uuid]);

  const handleRemove = async (filename: string) => {
    if (!uuid) return;
    await uninstallMod(uuid, filename);
    setInstalled(installed.filter(m => m.filename !== filename));
  };

  return (
    <div>
      <h3 className="font-semibold mb-2">Installed Mods</h3>
      {installed.length === 0 && <div>No mods installed.</div>}
      {installed.map((mod) => (
        <div key={mod.filename} className="border-b py-2 flex justify-between items-center">
          <div>
            <div className="font-bold">{mod.project_id}</div>
            <div className="text-xs text-gray-600">{mod.version_id}</div>
            <div className="text-xs">{mod.filename}</div>
          </div>
          <button className="btn btn-xs btn-error" onClick={() => handleRemove(mod.filename)}>
            Remove
          </button>
        </div>
      ))}
    </div>
  );
};

export default ModsInstalled;