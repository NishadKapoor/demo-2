import { StrictMode, Suspense } from 'react';
import { BrowserRouter } from 'react-router-dom';
import ReactDOM from 'react-dom/client';

import Layout from '@/layout';
import './main.scss';
//test
ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <StrictMode>
    <Suspense fallback={null}>
      <BrowserRouter>
        <Layout />
      </BrowserRouter>
    </Suspense>
  </StrictMode>,
);
