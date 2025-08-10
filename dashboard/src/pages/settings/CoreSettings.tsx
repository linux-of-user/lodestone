import React, { useState } from 'react';
import { openTcpPort, openUdpPort, closeTcpPort, closeUdpPort, getExternalIp } from '../../utils/apis';
import { toast } from 'react-toastify';

const CoreSettings: React.FC = () => {
  const [port, setPort] = useState('');
  const [externalIp, setExternalIp] = useState<string | null>(null);

  const handleOpenTcp = async () => {
    try {
      await openTcpPort(Number(port));
      toast.success('Opened TCP port!');
    } catch (e) {
      toast.error('Failed to open TCP port');
    }
  };
  const handleOpenUdp = async () => {
    try {
      await openUdpPort(Number(port));
      toast.success('Opened UDP port!');
    } catch (e) {
      toast.error('Failed to open UDP port');
    }
  };
  const handleCloseTcp = async () => {
    try {
      await closeTcpPort(Number(port));
      toast.success('Closed TCP port!');
    } catch (e) {
      toast.error('Failed to close TCP port');
    }
  };
  const handleCloseUdp = async () => {
    try {
      await closeUdpPort(Number(port));
      toast.success('Closed UDP port!');
    } catch (e) {
      toast.error('Failed to close UDP port');
    }
  };
  const handleGetIp = async () => {
    try {
      const ip = await getExternalIp();
      setExternalIp(ip);
      toast.success('Fetched external IP!');
    } catch (e) {
      toast.error('Failed to get external IP');
    }
  };

  return (
    <div>
      <h2 className="font-semibold mb-2">UPnP Port Management</h2>
      <div className="flex gap-2 items-center mb-2">
        <input className="border px-2 py-1 rounded w-24" placeholder="Port" value={port} onChange={e => setPort(e.target.value)} />
        <button className="btn btn-xs btn-primary" onClick={handleOpenTcp}>Open TCP</button>
        <button className="btn btn-xs btn-primary" onClick={handleOpenUdp}>Open UDP</button>
        <button className="btn btn-xs btn-warning" onClick={handleCloseTcp}>Close TCP</button>
        <button className="btn btn-xs btn-warning" onClick={handleCloseUdp}>Close UDP</button>
        <button className="btn btn-xs btn-info" onClick={handleGetIp}>Get External IP</button>
      </div>
      {externalIp && <div>External IP: <span className="font-mono">{externalIp}</span></div>}
    </div>
  );
};

export default CoreSettings;