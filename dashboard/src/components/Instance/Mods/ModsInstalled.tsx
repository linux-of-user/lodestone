import React, { useEffect, useState } from 'react';
import { listInstalledMods, uninstallMod } from '../../../utils/apis';

const ModsInstalled: React.FC<{ uuid?: string }> = ({ uuid }) => {
  const [installed, setInstalled] = useState<any[]>([]);

  useEffect(() => {
    if (!uuid) return;
    listInstalledMods(uuid).then(setInstalled);
  }, [uuid]);

  const handleRemove = async (fileName: string) => {
    if (!uuid) return;
    await uninstallMod(uuid, fileName);
    setInstalled(installed.filter(m => m.file_name !== fileName));
  };

  return (
    <div>
      <h3 className="font-semibold mb-2">Installed Mods</h3>
      {installed.length === 0 && <div>No mods installed.</div>}
      {installed.map((mod) => (
        <div key={mod.file_name} className="border-b py-2 flex justify-between items-center">
          <div>
            <div className="font-bold">{mod.project_id}</div>
            <div className="text-xs text-gray-600">{mod.version_id}</div>
          </div>
          <button className="btn btn-xs btn-error" onClick={() => handleRemove(mod.file_name)}>
            Remove
          </button>
        </div>
      ))}
    </div>
  );
};

export default ModsInstalled;