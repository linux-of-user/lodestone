import React, { useEffect, useState } from 'react';
import {
  listDockerContainers,
  startDockerContainer,
  stopDockerContainer,
  restartDockerContainer,
  killDockerContainer,
  getDockerContainerLogs,
} from '../../utils/apis';
import { toast } from 'react-toastify';

const DockerPage: React.FC = () => {
  const [containers, setContainers] = useState<any[]>([]);
  const [logs, setLogs] = useState<string[] | null>(null);

  useEffect(() => {
    listDockerContainers().then(setContainers);
  }, []);

  const handleAction = async (id: string, action: 'start' | 'stop' | 'restart' | 'kill') => {
    try {
      if (action === 'start') await startDockerContainer(id);
      if (action === 'stop') await stopDockerContainer(id);
      if (action === 'restart') await restartDockerContainer(id);
      if (action === 'kill') await killDockerContainer(id);
      toast.success(`${action}ed container`);
      listDockerContainers().then(setContainers);
    } catch (e) {
      toast.error(`Failed to ${action} container`);
    }
  };

  const handleLogs = async (id: string) => {
    const data = await getDockerContainerLogs(id, 100);
    setLogs(data);
  };

  return (
    <div className="p-4">
      <h2 className="font-bold mb-4">Docker Containers</h2>
      <table className="table-auto w-full mb-4">
        <thead>
          <tr>
            <th>Name</th>
            <th>State</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {containers.map((c) => (
            <tr key={c.uuid}>
              <td>{c.name}</td>
              <td>{c.state}</td>
              <td>
                <button className="btn btn-xs btn-success mr-2" onClick={() => handleAction(c.uuid, 'start')}>Start</button>
                <button className="btn btn-xs btn-warning mr-2" onClick={() => handleAction(c.uuid, 'stop')}>Stop</button>
                <button className="btn btn-xs btn-info mr-2" onClick={() => handleAction(c.uuid, 'restart')}>Restart</button>
                <button className="btn btn-xs btn-error mr-2" onClick={() => handleAction(c.uuid, 'kill')}>Kill</button>
                <button className="btn btn-xs btn-secondary" onClick={() => handleLogs(c.uuid)}>Logs</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {logs && (
        <div className="bg-gray-900 text-white p-4 rounded mb-4">
          <h3 className="font-semibold mb-2">Container Logs</h3>
          <pre className="overflow-x-auto whitespace-pre-wrap">{logs.join('\n')}</pre>
          <button className="btn btn-xs btn-primary mt-2" onClick={() => setLogs(null)}>Close</button>
        </div>
      )}
    </div>
  );
};

export default DockerPage;